use base64::Engine;
use reqwest::Client;
use rsa::pss::SigningKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::RsaPrivateKey;
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;

const KALSHI_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";

#[derive(Debug, Deserialize)]
pub struct KalshiOrderResponse {
    pub order: Option<Value>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

/// Submit a limit order to Kalshi via REST API.
///
/// Auth: RSA-PSS signing (same pattern as ws.rs WebSocket handshake).
///
/// Translation from simulation:
/// - "sell YES at X" on Kalshi = "buy NO at (1-X)"
/// - Submit one aggressive limit order at the worst price from the simulation.
///   The exchange fills at best available prices up to the limit.
pub async fn submit_order(
    client: &Client,
    api_key: &str,
    private_key: &RsaPrivateKey,
    ticker: &str,
    side: &str,        // "yes" or "no"
    action: &str,      // "buy" or "sell"
    price_cents: u32,  // limit price in cents (1-99)
    count: u32,        // number of contracts
) -> Result<KalshiOrderResponse, String> {
    let path = "/trade-api/v2/portfolio/orders";
    let url = format!("{}/portfolio/orders", KALSHI_BASE);

    // RSA-PSS signing (same pattern as ws.rs lines 115-125)
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();
    let sign_message = format!("{}POST{}", timestamp_ms, path);
    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let mut rng = rsa::rand_core::OsRng;
    let signature = signing_key.sign_with_rng(&mut rng, sign_message.as_bytes());
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    // Build order body
    let mut body = serde_json::json!({
        "ticker": ticker,
        "side": side,
        "action": action,
        "type": "limit",
        "count": count,
    });

    // Set price field based on side
    if side == "yes" {
        body["yes_price"] = serde_json::json!(price_cents);
    } else {
        body["no_price"] = serde_json::json!(price_cents);
    }

    eprintln!(
        "[EXEC] Kalshi order: {} {} {} count={} price={}c",
        action, side, ticker, count, price_cents,
    );

    let resp = client
        .post(&url)
        .header("KALSHI-ACCESS-KEY", api_key)
        .header("KALSHI-ACCESS-SIGNATURE", &sig_b64)
        .header("KALSHI-ACCESS-TIMESTAMP", &timestamp_ms)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Kalshi order request failed: {}", e))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read response body failed: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "Kalshi order rejected: HTTP {} — {}",
            status, text
        ));
    }

    serde_json::from_str(&text)
        .map_err(|e| format!("Parse Kalshi response failed: {} — raw: {}", e, text))
}
