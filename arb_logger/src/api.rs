use crate::types::{ExecutionOverhead, MatchedMarket};
use reqwest::Client;
use serde_json::Value;

const KALSHI_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";
const POLYMARKET_CLOB: &str = "https://clob.polymarket.com";
const POLYMARKET_GAMMA: &str = "https://gamma-api.polymarket.com";

const BELLWETHER_MARKETS_URL: &str = "https://bellwethermetrics.com/data/active_markets.json";
const MIN_SPREAD: f64 = 0.02;
const MAX_SPREAD: f64 = 0.25;

/// Markets manually flagged as mismatched (question mismatch between platforms).
const EXCLUDED_TICKERS: &[&str] = &[
    "BWR-BOC-HIKE-OVERNIGHT_RATE-SPECIFIC_MEETING-25BPS-APR2026",
    "BWR-ARMENIA_ALLIANCE-WIN-PARLIAMENT_AM-CERTIFIED-ANY-2026",
    "BWR-KATAEB-WIN-PARLIAMENT_LB-CERTIFIED-ANY-2026",
    "BWR-BOC-CUT-OVERNIGHT_RATE-SPECIFIC_MEETING-25BPS-APR2026",
    "BWR-PARK-WIN-MAYOR_SEOUL-CERTIFIED-ANY-2026",
    "BWR-GOP-WIN-HOUSE_OK_05-CERTIFIED-ANY-2026",
];

/// Fetch all matched markets from Bellwether, filtered to 2%-25% cross-platform spread.
pub async fn get_tracked_markets(client: &Client) -> Result<Vec<MatchedMarket>, String> {
    let resp: Value = client
        .get(BELLWETHER_MARKETS_URL)
        .send()
        .await
        .map_err(|e| format!("Bellwether fetch failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Bellwether parse failed: {}", e))?;

    let markets = resp["markets"]
        .as_array()
        .ok_or_else(|| "Bellwether response missing 'markets' array".to_string())?;

    let mut result = Vec::new();
    for m in markets {
        if !m["has_both"].as_bool().unwrap_or(false) {
            continue;
        }
        let k_ticker = match m["k_ticker"].as_str() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        let pm_token = match m["pm_token_id"].as_str() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        let ticker = m["ticker"].as_str().unwrap_or("");
        if EXCLUDED_TICKERS.contains(&ticker) {
            continue;
        }
        let spread = m["spread"].as_f64().unwrap_or(0.0).abs();
        if spread >= MIN_SPREAD && spread <= MAX_SPREAD {
            result.push(MatchedMarket {
                ticker: m["ticker"].as_str().unwrap_or("").to_string(),
                kalshi_ticker: Some(k_ticker.to_string()),
                polymarket_token: Some(pm_token.to_string()),
            });
        }
    }

    Ok(result)
}


/// Fetch Polymarket fee rates for all tokens concurrently.
/// Returns a map of token_id → base_fee_rate (e.g. 0.04 for 4% politics markets).
pub async fn fetch_polymarket_fee_rates(
    client: &Client,
    token_ids: &[String],
) -> std::collections::HashMap<String, f64> {
    use futures_util::future::join_all;

    let futs: Vec<_> = token_ids
        .iter()
        .map(|token| {
            let client = client.clone();
            let token = token.clone();
            async move {
                let url = format!("{}/fee-rate?token_id={}", POLYMARKET_CLOB, token);
                let rate = match client.get(&url).send().await {
                    Ok(resp) => match resp.json::<Value>().await {
                        Ok(json) => json["base_fee"]
                            .as_f64()
                            .or_else(|| {
                                json["base_fee"]
                                    .as_str()
                                    .and_then(|s| s.parse::<f64>().ok())
                            })
                            .unwrap_or(0.0),
                        Err(_) => 0.0,
                    },
                    Err(_) => 0.0,
                };
                (token, rate)
            }
        })
        .collect();

    let results: std::collections::HashMap<String, f64> =
        join_all(futs).await.into_iter().collect();
    results
}

/// Fetch resolution dates from both native APIs concurrently.
pub async fn fetch_resolution_date(
    client: &Client,
    kalshi_ticker: Option<&str>,
    polymarket_token: Option<&str>,
) -> Result<Option<String>, String> {
    let kalshi_fut = async {
        if let Some(ticker) = kalshi_ticker {
            let url = format!("{}/markets/{}", KALSHI_BASE, ticker);
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(json) = resp.json::<Value>().await {
                    return json["market"]["close_time"]
                        .as_str()
                        .map(|s| s.to_string());
                }
            }
        }
        None
    };

    let poly_fut = async {
        if let Some(token) = polymarket_token {
            let url = format!("{}/markets?clob_token_ids={}", POLYMARKET_GAMMA, token);
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(arr) = json.as_array() {
                        if let Some(first) = arr.first() {
                            return first["end_date_iso"]
                                .as_str()
                                .map(|s| s.to_string());
                        }
                    }
                }
            }
        }
        None
    };

    let (kalshi_date, poly_date) = tokio::join!(kalshi_fut, poly_fut);
    Ok(kalshi_date.or(poly_date))
}

/// Measure execution overhead at startup. Constructs but does NOT send orders.
/// Times request construction and measures network latency via lightweight GETs.
pub async fn measure_execution_overhead(client: &Client) -> ExecutionOverhead {
    // Measure Kalshi request construction overhead
    // (build a full authenticated order body — just serialization + header construction)
    let kalshi_start = std::time::Instant::now();
    let _kalshi_body = serde_json::json!({
        "action": "buy",
        "type": "limit",
        "side": "yes",
        "count": 1,
        "yes_price": 50,
        "ticker": "DRY-RUN-TIMING",
    });
    let _kalshi_req = client
        .post(format!("{}/portfolio/orders", KALSHI_BASE))
        .bearer_auth("dry-run-token")
        .json(&_kalshi_body)
        .build();
    let kalshi_construction_ms = kalshi_start.elapsed().as_millis() as u64;

    // Measure Polymarket request construction overhead
    // (build EIP-712 order struct — just serialization, no wallet signing available)
    let poly_start = std::time::Instant::now();
    let _poly_body = serde_json::json!({
        "order": {
            "salt": "0",
            "maker": "0x0000000000000000000000000000000000000000",
            "signer": "0x0000000000000000000000000000000000000000",
            "taker": "0x0000000000000000000000000000000000000000",
            "tokenId": "0",
            "makerAmount": "1000000",
            "takerAmount": "500000",
            "side": "BUY",
            "expiration": "0",
            "nonce": "0",
            "feeRateBps": "0",
            "signatureType": 0,
        }
    });
    let _poly_req = client
        .post(format!("{}/order", POLYMARKET_CLOB))
        .json(&_poly_body)
        .build();
    let polymarket_construction_ms = poly_start.elapsed().as_millis() as u64;

    // Measure Kalshi network latency (time a lightweight GET)
    let kalshi_network_ms = measure_network_latency(
        client,
        &format!("{}/exchange/status", KALSHI_BASE),
    )
    .await;

    // Measure Polymarket network latency
    let polymarket_network_ms = measure_network_latency(
        client,
        &format!("{}/price?token_id=1", POLYMARKET_CLOB),
    )
    .await;

    let vpn_overhead_ms: u64 = std::env::var("VPN_OVERHEAD_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let overhead = ExecutionOverhead {
        kalshi_construction_ms,
        polymarket_construction_ms,
        kalshi_network_ms,
        polymarket_network_ms,
        polygon_settlement_ms: 2500, // conservative estimate: 2-3s on-chain
        vpn_overhead_ms,
    };

    overhead
}

/// Time a GET request to measure network latency to a given endpoint.
async fn measure_network_latency(client: &Client, url: &str) -> u64 {
    let start = std::time::Instant::now();
    // We only care about time-to-response, not the response content
    let _ = client.get(url).send().await;
    start.elapsed().as_millis() as u64
}
