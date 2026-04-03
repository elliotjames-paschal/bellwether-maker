use crate::orderbook;
use crate::types::{FeeRates, MatchedMarket, PlatformBook};
use futures_util::future::join_all;
use ordered_float::OrderedFloat;
use reqwest::Client;
use serde_json::Value;
use std::fmt;

const KALSHI_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";
const POLYMARKET_CLOB: &str = "https://clob.polymarket.com";

const MIN_NEV_PER_SHARE: f64 = 0.03;
const MIN_POSITION: f64 = 200.0;

#[derive(Debug)]
enum AuditVerdict {
    RealArb,
    SpreadOnly,
    Illiquid,
    NoSpread,
    FetchError,
}

impl fmt::Display for AuditVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditVerdict::RealArb => write!(f, "REAL ARB"),
            AuditVerdict::SpreadOnly => write!(f, "SPREAD ONLY"),
            AuditVerdict::Illiquid => write!(f, "ILLIQUID"),
            AuditVerdict::NoSpread => write!(f, "NO SPREAD"),
            AuditVerdict::FetchError => write!(f, "FETCH ERROR"),
        }
    }
}

struct AuditResult {
    ticker: String,
    cross_spread: Option<f64>,
    kalshi_internal: Option<f64>,
    pm_internal: Option<f64>,
    net_profit: Option<f64>,
    depth: Option<f64>,
    nev: Option<f64>,
    verdict: AuditVerdict,
    reason: String,
}

pub async fn run_audit(markets: &[MatchedMarket], fees: &FeeRates) {
    let client = Client::new();

    let futures: Vec<_> = markets
        .iter()
        .map(|market| {
            let client = client.clone();
            let fees = fees.clone();
            let ticker = market.ticker.clone();
            let kalshi_ticker = market.kalshi_ticker.clone();
            let pm_token = market.polymarket_token.clone();
            async move {
                let pm_token_str = pm_token.as_deref().unwrap_or("");
                let (k_result, pm_result) = tokio::join!(
                    fetch_kalshi_book(&client, kalshi_ticker.as_deref().unwrap_or("")),
                    fetch_polymarket_book(&client, pm_token_str),
                );

                match (k_result, pm_result) {
                    (Ok(k_book), Ok(pm_book)) => {
                        compute_audit_metrics(&ticker, pm_token_str, &k_book, &pm_book, &fees)
                    }
                    (Err(e), _) => AuditResult {
                        ticker,
                        cross_spread: None,
                        kalshi_internal: None,
                        pm_internal: None,
                        net_profit: None,
                        depth: None,
                        nev: None,
                        verdict: AuditVerdict::FetchError,
                        reason: format!("Kalshi: {}", e),
                    },
                    (_, Err(e)) => AuditResult {
                        ticker,
                        cross_spread: None,
                        kalshi_internal: None,
                        pm_internal: None,
                        net_profit: None,
                        depth: None,
                        nev: None,
                        verdict: AuditVerdict::FetchError,
                        reason: format!("Polymarket: {}", e),
                    },
                }
            }
        })
        .collect();

    let results = join_all(futures).await;
    print_audit_table(&results);
}

pub(crate) async fn fetch_kalshi_book(client: &Client, ticker: &str) -> Result<PlatformBook, String> {
    let url = format!("{}/markets/{}/orderbook", KALSHI_BASE, ticker);
    let resp: Value = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("parse failed: {}", e))?;

    let mut book = PlatformBook::new();

    // YES bids from orderbook_fp.yes_dollars
    if let Some(levels) = resp["orderbook_fp"]["yes_dollars"].as_array() {
        for level in levels {
            if let Some(arr) = level.as_array() {
                if arr.len() >= 2 {
                    let price = arr[0]
                        .as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let size = arr[1]
                        .as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    if size > 0.0 {
                        book.bids.insert(OrderedFloat(price), size);
                    }
                }
            }
        }
    }

    // NO bids → YES asks (YES ask = 1.0 - NO bid price)
    if let Some(levels) = resp["orderbook_fp"]["no_dollars"].as_array() {
        for level in levels {
            if let Some(arr) = level.as_array() {
                if arr.len() >= 2 {
                    let no_price = arr[0]
                        .as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let size = arr[1]
                        .as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let yes_ask_price = 1.0 - no_price;
                    if size > 0.0 && yes_ask_price > 0.0 {
                        book.asks.insert(OrderedFloat(yes_ask_price), size);
                    }
                }
            }
        }
    }

    Ok(book)
}

pub(crate) async fn fetch_polymarket_book(client: &Client, token_id: &str) -> Result<PlatformBook, String> {
    let url = format!("{}/book?token_id={}", POLYMARKET_CLOB, token_id);
    let resp: Value = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("parse failed: {}", e))?;

    let mut book = PlatformBook::new();

    if let Some(bids) = resp["bids"].as_array() {
        for level in bids {
            let price = level["price"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let size = level["size"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            if size > 0.0 {
                book.bids.insert(OrderedFloat(price), size);
            }
        }
    }

    if let Some(asks) = resp["asks"].as_array() {
        for level in asks {
            let price = level["price"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let size = level["size"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            if size > 0.0 {
                book.asks.insert(OrderedFloat(price), size);
            }
        }
    }

    Ok(book)
}

fn compute_audit_metrics(
    ticker: &str,
    pm_token: &str,
    k_book: &PlatformBook,
    pm_book: &PlatformBook,
    fees: &FeeRates,
) -> AuditResult {
    // Internal spreads
    let kalshi_internal = best_internal_spread(k_book);
    let pm_internal = best_internal_spread(pm_book);

    // Cross-platform spread (best of both directions)
    // Dir A: buy YES on PM asks, sell YES on Kalshi bids
    let dir_a = match (k_book.bids.keys().next_back(), pm_book.asks.keys().next()) {
        (Some(k_bid), Some(pm_ask)) => k_bid.into_inner() - pm_ask.into_inner(),
        _ => f64::NEG_INFINITY,
    };
    // Dir B: buy YES on Kalshi asks, sell YES on PM bids
    let dir_b = match (pm_book.bids.keys().next_back(), k_book.asks.keys().next()) {
        (Some(pm_bid), Some(k_ask)) => pm_bid.into_inner() - k_ask.into_inner(),
        _ => f64::NEG_INFINITY,
    };
    let cross_spread = dir_a.max(dir_b);

    if cross_spread <= 0.0 {
        return AuditResult {
            ticker: short_ticker(ticker),
            cross_spread: Some(cross_spread),
            kalshi_internal,
            pm_internal,
            net_profit: None,
            depth: None,
            nev: None,
            verdict: AuditVerdict::NoSpread,
            reason: "No cross-platform spread".into(),
        };
    }

    // Run full round-trip simulation
    match orderbook::simulate_round_trip_btree(k_book, pm_book, fees.kalshi_fee, fees.pm_rate(pm_token)) {
        Some((rt, detail)) => {
            let depth = rt.total_entry_cost;
            let nev = detail.nev_per_share;
            let net = detail.net_profit;

            let verdict = if nev >= MIN_NEV_PER_SHARE && depth >= MIN_POSITION {
                AuditVerdict::RealArb
            } else if nev >= MIN_NEV_PER_SHARE {
                AuditVerdict::Illiquid
            } else {
                AuditVerdict::SpreadOnly
            };

            let reason = if matches!(verdict, AuditVerdict::RealArb) {
                format!(
                    "{}: buy {} sell {}",
                    detail.direction,
                    detail.entry_leg.platform,
                    detail.exit_leg.platform,
                )
            } else if matches!(verdict, AuditVerdict::Illiquid) {
                format!("depth ${:.0} < ${:.0} min", depth, MIN_POSITION)
            } else {
                format!("nev {:.1}c < {:.1}c min", nev * 100.0, MIN_NEV_PER_SHARE * 100.0)
            };

            AuditResult {
                ticker: short_ticker(ticker),
                cross_spread: Some(cross_spread),
                kalshi_internal,
                pm_internal,
                net_profit: Some(net),
                depth: Some(depth),
                nev: Some(nev),
                verdict,
                reason,
            }
        }
        None => AuditResult {
            ticker: short_ticker(ticker),
            cross_spread: Some(cross_spread),
            kalshi_internal,
            pm_internal,
            net_profit: None,
            depth: None,
            nev: None,
            verdict: AuditVerdict::SpreadOnly,
            reason: "Spread exists but no profitable round-trip after fees".into(),
        },
    }
}

fn best_internal_spread(book: &PlatformBook) -> Option<f64> {
    match (book.bids.keys().next_back(), book.asks.keys().next()) {
        (Some(best_bid), Some(best_ask)) => {
            Some(best_ask.into_inner() - best_bid.into_inner())
        }
        _ => None,
    }
}

/// Shorten BWR-style tickers for display.
/// BWR-DEM-WIN-HOUSE_GA-02-CERTIFIED-ANY-2026 → HOUSE_GA-02
/// BWR-FED-CUT-FFR-SPECIFIC_MEETING-25BPS-JUL2026 → FFR-25BPS
fn short_ticker(ticker: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if parts.len() >= 5 {
        if let Some(idx) = parts.iter().position(|p| {
            p.starts_with("HOUSE") || p.starts_with("GOV") || p.starts_with("SENATE")
        }) {
            // Include next part if it looks like a district number (e.g. "02", "15")
            if idx + 1 < parts.len() {
                let next = parts[idx + 1];
                if next.len() <= 3 && next.chars().all(|c| c.is_ascii_digit()) {
                    return format!("{}-{}", parts[idx], next);
                }
            }
            return parts[idx].to_string();
        }
        return parts[3..parts.len().saturating_sub(2)].join("-");
    }
    ticker.to_string()
}

fn print_audit_table(results: &[AuditResult]) {
    // Header
    println!();
    println!(
        "{:<20} {:>9} {:>9} {:>9} {:>8} {:>8} {:>8}  {}",
        "Market", "X-spread", "K-intrnl", "PM-intrnl", "Net $", "Depth $", "NEV", "Verdict"
    );
    println!("{}", "-".repeat(100));

    for r in results {
        let cross = r
            .cross_spread
            .map(|v| format!("{:>7.1}c", v * 100.0))
            .unwrap_or_else(|| "    --".into());
        let ki = r
            .kalshi_internal
            .map(|v| format!("{:>7.1}c", v * 100.0))
            .unwrap_or_else(|| "    --".into());
        let pi = r
            .pm_internal
            .map(|v| format!("{:>7.1}c", v * 100.0))
            .unwrap_or_else(|| "    --".into());
        let net = r
            .net_profit
            .map(|v| format!("{:>6.2}", v))
            .unwrap_or_else(|| "    --".into());
        let depth = r
            .depth
            .map(|v| format!("{:>6.0}", v))
            .unwrap_or_else(|| "    --".into());
        let nev = r
            .nev
            .map(|v| format!("{:>6.1}c", v * 100.0))
            .unwrap_or_else(|| "    --".into());

        println!(
            "{:<20} {:>9} {:>9} {:>9} {:>8} {:>8} {:>8}  {} {}",
            r.ticker, cross, ki, pi, net, depth, nev, r.verdict, r.reason,
        );
    }

    // Summary
    println!("{}", "-".repeat(100));

    let real = results
        .iter()
        .filter(|r| matches!(r.verdict, AuditVerdict::RealArb))
        .count();
    let spread_only = results
        .iter()
        .filter(|r| matches!(r.verdict, AuditVerdict::SpreadOnly))
        .count();
    let illiquid = results
        .iter()
        .filter(|r| matches!(r.verdict, AuditVerdict::Illiquid))
        .count();
    let no_spread = results
        .iter()
        .filter(|r| matches!(r.verdict, AuditVerdict::NoSpread))
        .count();
    let fetch_err = results
        .iter()
        .filter(|r| matches!(r.verdict, AuditVerdict::FetchError))
        .count();

    println!(
        "REAL ARB: {}  SPREAD ONLY: {}  ILLIQUID: {}  NO SPREAD: {}  FETCH ERROR: {}",
        real, spread_only, illiquid, no_spread, fetch_err,
    );

    if real > 0 {
        println!(
            "\nRecommendation: {} market(s) show executable arb opportunities. Kalshi WS subscription justified.",
            real,
        );
    } else if spread_only + illiquid > 0 {
        println!(
            "\nRecommendation: Spreads exist but none pass executability thresholds (NEV >= {}c, depth >= ${:.0}). Review market selection.",
            MIN_NEV_PER_SHARE * 100.0,
            MIN_POSITION,
        );
    } else {
        println!("\nRecommendation: No cross-platform spreads found. Do not subscribe to Kalshi WS.");
    }
    println!();
}
