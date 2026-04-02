use crate::types::{ExecutionOverhead, MatchedMarket};
use reqwest::Client;
use serde_json::Value;

const KALSHI_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";
const POLYMARKET_CLOB: &str = "https://clob.polymarket.com";
const POLYMARKET_GAMMA: &str = "https://gamma-api.polymarket.com";

/// Curated list of 13 markets selected for paper trading.
/// Selected based on: spread >= 3¢, cost_to_move_5c $200-$5000, reportability=fragile.
pub fn get_tracked_markets() -> Vec<MatchedMarket> {
    vec![
        MatchedMarket {
            ticker: "BWR-FED-CUT-FFR-SPECIFIC_MEETING-25BPS-JUL2026".into(),
            kalshi_ticker: Some("KXFEDDECISION-26JUL-C25".into()),
            polymarket_token: Some("107111506004559425167535337910569809007100298896139468418632748246491535011700".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_OH_15-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-OH15-26-R".into()),
            polymarket_token: Some("97813253986291805301845546476916155582068629376011287079676633578166768758974".into()),
        },
        MatchedMarket {
            ticker: "BWR-PELTOLA-WIN-SENATE_AK-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXAKSENATE-26NOV03-MPEL".into()),
            polymarket_token: Some("72139607190315861034411702195742538105117960260100800877125381826122747504258".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_TN07-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-TN07-26-R".into()),
            polymarket_token: Some("108435435156169780511802105123559082298215299373504251103377970956623189780379".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_GA07-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-GA07-26-R".into()),
            polymarket_token: Some("40175477873205534291803272100440407729292650174524862584432986239438169710120".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-GOV_ME-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("GOVPARTYME-26-R".into()),
            polymarket_token: Some("77900872247159891045693614974240851672235710405168744753050508553885061962390".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-GOV_GA-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("GOVPARTYGA-26-R".into()),
            polymarket_token: Some("17555504943162344098534978138053308162738914357892515623100381936995379845407".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_SC07-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-SC07-26-R".into()),
            polymarket_token: Some("96580445079668683787368085070582364594702719841272426188104847420199646364705".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_TX36-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-TX36-26-R".into()),
            polymarket_token: Some("28447198635554099262973905217726367470243680945205937032068258346722106906223".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_FL02-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-FL02-26-R".into()),
            polymarket_token: Some("54702326895469204251772029072335229338384845206963136614108005629884908128243".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_FL21-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-FL21-26-R".into()),
            polymarket_token: Some("7460473200203500049888783719604614089184190030851720364867764732498211690017".into()),
        },
        MatchedMarket {
            ticker: "BWR-DEM-WIN-HOUSE_GA-02-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-GA02-26-D".into()),
            polymarket_token: Some("19606361507228583496697532968311671202824580545845457919639287792278838929378".into()),
        },
        MatchedMarket {
            ticker: "BWR-GOP-WIN-HOUSE_GA-12-CERTIFIED-ANY-2026".into(),
            kalshi_ticker: Some("KXHOUSERACE-GA12-26-R".into()),
            polymarket_token: Some("101804221129923221311468014764974740374334424163195744719593146528253694482543".into()),
        },
    ]
}


/// Fetch Polymarket fee rate.
pub async fn fetch_polymarket_fee_rate(client: &Client) -> Result<f64, String> {
    let url = format!("{}/fee-rate", POLYMARKET_CLOB);
    let resp: Value = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("fetch_polymarket_fee_rate request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("fetch_polymarket_fee_rate parse failed: {}", e))?;

    let fee = resp["taker_base_fee"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| resp["taker_base_fee"].as_f64())
        .unwrap_or(0.0);

    Ok(fee)
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
