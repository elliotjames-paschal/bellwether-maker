use base64::Engine;
use ethers_core::abi::{encode, Token};
use ethers_core::types::{Address, H256, U256};
use ethers_core::utils::keccak256;
use ethers_signers::{LocalWallet, Signer};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

const POLYMARKET_CLOB: &str = "https://clob.polymarket.com";
const CHAIN_ID: u64 = 137;
const NEG_RISK_CTF_EXCHANGE: &str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";
const CTF_EXCHANGE: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
const CREDS_FILE: &str = "polymarket_creds.json";

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketCreds {
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct PolymarketOrderResponse {
    #[serde(rename = "orderID")]
    pub order_id: Option<String>,
    pub success: Option<bool>,
    #[serde(rename = "errorMsg")]
    pub error_msg: Option<String>,
    #[serde(rename = "transactionsHashes")]
    pub transaction_hashes: Option<Vec<String>>,
}

/// Load cached creds from file, or derive new ones via L1 auth.
pub async fn load_or_derive_creds(
    client: &Client,
    wallet: &LocalWallet,
) -> Result<PolymarketCreds, String> {
    let address = format!("{:#x}", wallet.address());

    // Try loading from cache
    if let Ok(contents) = std::fs::read_to_string(CREDS_FILE) {
        if let Ok(creds) = serde_json::from_str::<PolymarketCreds>(&contents) {
            if creds.address.to_lowercase() == address.to_lowercase() {
                eprintln!("[EXEC] Polymarket creds loaded from {}", CREDS_FILE);
                return Ok(creds);
            }
            eprintln!(
                "[EXEC] Cached creds address mismatch (cached={}, wallet={}), re-deriving",
                creds.address, address
            );
        }
    }

    // Derive new creds via L1 auth
    eprintln!("[EXEC] Deriving Polymarket API key via L1 auth...");
    let creds = derive_api_key(client, wallet).await?;

    // Cache to file
    let json = serde_json::to_string_pretty(&creds)
        .map_err(|e| format!("Serialize creds failed: {}", e))?;
    std::fs::write(CREDS_FILE, &json)
        .map_err(|e| format!("Write {} failed: {}", CREDS_FILE, e))?;
    eprintln!("[EXEC] Polymarket creds cached to {}", CREDS_FILE);

    Ok(creds)
}

// ---------------------------------------------------------------------------
// L1 Auth — EIP-712 ClobAuth signing to derive API key
// ---------------------------------------------------------------------------

async fn derive_api_key(
    client: &Client,
    wallet: &LocalWallet,
) -> Result<PolymarketCreds, String> {
    let address = wallet.address();
    let timestamp = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    let nonce = U256::zero();
    let message = "This message attests that I am the owner of the attached address.";

    // EIP-712 domain separator (no verifyingContract)
    let domain_sep = domain_separator_no_contract("ClobAuthDomain", "1", CHAIN_ID);

    // Struct hash
    let type_hash = keccak256(
        b"ClobAuth(address address,string timestamp,uint256 nonce,string message)",
    );
    let struct_hash = keccak256(&encode(&[
        Token::FixedBytes(type_hash.to_vec()),
        Token::Address(address),
        Token::FixedBytes(keccak256(timestamp.as_bytes()).to_vec()),
        Token::Uint(nonce),
        Token::FixedBytes(keccak256(message.as_bytes()).to_vec()),
    ]));

    // EIP-712 digest
    let digest = eip712_digest(domain_sep, struct_hash);

    // Sign
    let sig = wallet
        .sign_hash(digest)
        .map_err(|e| format!("Sign ClobAuth failed: {}", e))?;
    let sig_hex = format!("0x{}", hex::encode(sig.to_vec()));

    // POST to derive endpoint
    let body = serde_json::json!({
        "address": format!("{:#x}", address),
        "signature": sig_hex,
        "timestamp": timestamp,
        "nonce": "0",
        "message": message,
    });

    let resp = client
        .post(format!("{}/auth/derive-api-key", POLYMARKET_CLOB))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Derive API key request failed: {}", e))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read derive response failed: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Derive API key failed: HTTP {} — {}",
            status, text
        ));
    }

    let json: Value =
        serde_json::from_str(&text).map_err(|e| format!("Parse derive response: {}", e))?;

    Ok(PolymarketCreds {
        api_key: json["apiKey"]
            .as_str()
            .ok_or("Missing apiKey in response")?
            .to_string(),
        secret: json["secret"]
            .as_str()
            .ok_or("Missing secret in response")?
            .to_string(),
        passphrase: json["passphrase"]
            .as_str()
            .ok_or("Missing passphrase in response")?
            .to_string(),
        address: format!("{:#x}", address),
    })
}

// ---------------------------------------------------------------------------
// L2 Auth — HMAC-SHA256 per-request signing
// ---------------------------------------------------------------------------

fn l2_headers(
    creds: &PolymarketCreds,
    method: &str,
    path: &str,
    body: &str,
) -> Result<Vec<(&'static str, String)>, String> {
    let timestamp = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    let msg = format!("{}{}{}{}", timestamp, method, path, body);

    let secret_bytes = base64::engine::general_purpose::STANDARD
        .decode(&creds.secret)
        .map_err(|e| format!("Decode HMAC secret: {}", e))?;

    let mut mac =
        HmacSha256::new_from_slice(&secret_bytes).map_err(|e| format!("HMAC init: {}", e))?;
    mac.update(msg.as_bytes());
    let signature =
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    Ok(vec![
        ("POLY_ADDRESS", creds.address.clone()),
        ("POLY_API_KEY", creds.api_key.clone()),
        ("POLY_SIGNATURE", signature),
        ("POLY_TIMESTAMP", timestamp),
        ("POLY_PASSPHRASE", creds.passphrase.clone()),
    ])
}

// ---------------------------------------------------------------------------
// Order submission
// ---------------------------------------------------------------------------

/// Submit a limit order to Polymarket CLOB.
///
/// `side`: 0 = BUY, 1 = SELL
/// `price`: limit price per share (0.0 to 1.0)
/// `size`: number of shares (human units, e.g. 375.0)
/// `fee_rate_bps`: fee rate in basis points (e.g. 400 for 4%)
/// `neg_risk`: true for political markets (uses NegRisk CTF Exchange)
pub async fn submit_order(
    client: &Client,
    wallet: &LocalWallet,
    creds: &PolymarketCreds,
    token_id: &str,
    side: u8,
    price: f64,
    size: f64,
    fee_rate_bps: u64,
    neg_risk: bool,
) -> Result<PolymarketOrderResponse, String> {
    let address = wallet.address();
    let exchange_addr: Address = if neg_risk {
        NEG_RISK_CTF_EXCHANGE
    } else {
        CTF_EXCHANGE
    }
    .parse()
    .map_err(|e| format!("Parse exchange address: {}", e))?;

    // Compute amounts in base units (6 decimals)
    let (maker_amount, taker_amount) = if side == 0 {
        // BUY: maker gives USDC, receives shares
        let maker = (size * price * 1e6).round() as u64;
        let taker = (size * 1e6).round() as u64;
        (maker, taker)
    } else {
        // SELL: maker gives shares, receives USDC
        let maker = (size * 1e6).round() as u64;
        let taker = (size * price * 1e6).round() as u64;
        (maker, taker)
    };

    // Random salt
    let mut salt_bytes = [0u8; 16];
    rsa::rand_core::RngCore::fill_bytes(&mut rsa::rand_core::OsRng, &mut salt_bytes);
    let salt = U256::from_big_endian(&salt_bytes);

    let token_id_u256 =
        U256::from_dec_str(token_id).map_err(|e| format!("Parse token_id: {}", e))?;

    // Build EIP-712 Order struct hash
    let order_type_hash = keccak256(
        b"Order(uint256 salt,address maker,address signer,address taker,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint256 expiration,uint256 nonce,uint256 feeRateBps,uint8 side,uint8 signatureType)",
    );

    let struct_hash = keccak256(&encode(&[
        Token::FixedBytes(order_type_hash.to_vec()),
        Token::Uint(salt),
        Token::Address(address),
        Token::Address(address), // signer = maker for EOA
        Token::Address(Address::zero()), // taker = 0x0 (any)
        Token::Uint(token_id_u256),
        Token::Uint(U256::from(maker_amount)),
        Token::Uint(U256::from(taker_amount)),
        Token::Uint(U256::zero()), // expiration = 0 (GTC)
        Token::Uint(U256::zero()), // nonce = 0
        Token::Uint(U256::from(fee_rate_bps)),
        Token::Uint(U256::from(side)),
        Token::Uint(U256::zero()), // signatureType = 0 (EOA)
    ]));

    // Domain separator (with verifyingContract)
    let domain_sep = domain_separator("ClobExchange", "1", CHAIN_ID, exchange_addr);

    // EIP-712 digest
    let digest = eip712_digest(domain_sep, struct_hash);

    // Sign
    let sig = wallet
        .sign_hash(digest)
        .map_err(|e| format!("Sign order failed: {}", e))?;
    let sig_hex = format!("0x{}", hex::encode(sig.to_vec()));

    let side_str = if side == 0 { "BUY" } else { "SELL" };

    // Build POST body
    let order_body = serde_json::json!({
        "order": {
            "salt": salt.to_string(),
            "maker": format!("{:#x}", address),
            "signer": format!("{:#x}", address),
            "taker": "0x0000000000000000000000000000000000000000",
            "tokenId": token_id,
            "makerAmount": maker_amount.to_string(),
            "takerAmount": taker_amount.to_string(),
            "expiration": "0",
            "nonce": "0",
            "feeRateBps": fee_rate_bps.to_string(),
            "side": side_str,
            "signatureType": 0,
            "signature": sig_hex,
        },
        "owner": format!("{:#x}", address),
        "orderType": "GTC",
    });

    eprintln!(
        "[EXEC] Polymarket order: {} token={} size={:.1} price={:.4} maker_amt={} taker_amt={}",
        side_str,
        &token_id[..20.min(token_id.len())],
        size,
        price,
        maker_amount,
        taker_amount,
    );

    let body_str = serde_json::to_string(&order_body)
        .map_err(|e| format!("Serialize order body: {}", e))?;

    let headers =
        l2_headers(creds, "POST", "/order", &body_str).map_err(|e| format!("L2 headers: {}", e))?;

    let mut req = client
        .post(format!("{}/order", POLYMARKET_CLOB))
        .header("Content-Type", "application/json")
        .body(body_str);

    for (name, value) in &headers {
        req = req.header(*name, value);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Polymarket order request failed: {}", e))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read PM response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Polymarket order rejected: HTTP {} — {}",
            status, text
        ));
    }

    serde_json::from_str(&text)
        .map_err(|e| format!("Parse PM response: {} — raw: {}", e, text))
}

// ---------------------------------------------------------------------------
// EIP-712 helpers
// ---------------------------------------------------------------------------

/// Domain separator WITHOUT verifyingContract (used for ClobAuth).
fn domain_separator_no_contract(name: &str, version: &str, chain_id: u64) -> [u8; 32] {
    let type_hash =
        keccak256(b"EIP712Domain(string name,string version,uint256 chainId)");
    keccak256(&encode(&[
        Token::FixedBytes(type_hash.to_vec()),
        Token::FixedBytes(keccak256(name.as_bytes()).to_vec()),
        Token::FixedBytes(keccak256(version.as_bytes()).to_vec()),
        Token::Uint(U256::from(chain_id)),
    ]))
}

/// Domain separator WITH verifyingContract (used for Order signing).
fn domain_separator(
    name: &str,
    version: &str,
    chain_id: u64,
    contract: Address,
) -> [u8; 32] {
    let type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    keccak256(&encode(&[
        Token::FixedBytes(type_hash.to_vec()),
        Token::FixedBytes(keccak256(name.as_bytes()).to_vec()),
        Token::FixedBytes(keccak256(version.as_bytes()).to_vec()),
        Token::Uint(U256::from(chain_id)),
        Token::Address(contract),
    ]))
}

/// Compute EIP-712 final digest: keccak256("\x19\x01" || domainSep || structHash).
fn eip712_digest(domain_sep: [u8; 32], struct_hash: [u8; 32]) -> H256 {
    let mut msg = Vec::with_capacity(66);
    msg.push(0x19);
    msg.push(0x01);
    msg.extend_from_slice(&domain_sep);
    msg.extend_from_slice(&struct_hash);
    H256(keccak256(&msg))
}

// ---------------------------------------------------------------------------
// Complementary token lookup
// ---------------------------------------------------------------------------

/// Fetch the NO token_id for a market given the YES token_id.
/// Uses the Polymarket Gamma API.
pub async fn fetch_no_token(client: &Client, yes_token_id: &str) -> Result<String, String> {
    let url = format!(
        "https://gamma-api.polymarket.com/markets?clob_token_ids={}",
        yes_token_id
    );
    let resp: Value = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Gamma API request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Gamma API parse failed: {}", e))?;

    let markets = resp
        .as_array()
        .ok_or("Gamma API: expected array")?;
    let market = markets
        .first()
        .ok_or("Gamma API: no market found for token")?;

    if let Some(tokens) = market["tokens"].as_array() {
        for token in tokens {
            let tid = token["token_id"].as_str().unwrap_or("");
            let outcome = token["outcome"].as_str().unwrap_or("");
            if outcome.eq_ignore_ascii_case("No") && !tid.is_empty() {
                return Ok(tid.to_string());
            }
        }
    }

    Err(format!(
        "Could not find NO token for YES token {}",
        yes_token_id
    ))
}

/// Check if a market uses neg_risk via the Gamma API.
pub async fn is_neg_risk(client: &Client, token_id: &str) -> bool {
    let url = format!(
        "https://gamma-api.polymarket.com/markets?clob_token_ids={}",
        token_id
    );
    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(json) = resp.json::<Value>().await {
            if let Some(arr) = json.as_array() {
                if let Some(market) = arr.first() {
                    return market["neg_risk"].as_bool().unwrap_or(true);
                }
            }
        }
    }
    true // default to neg_risk for political markets
}
