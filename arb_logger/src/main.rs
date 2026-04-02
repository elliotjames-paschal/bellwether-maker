// NOTE: Flash loan execution is a future consideration for atomic same-block
// execution on Polymarket -- both legs in one transaction, zero capital required.
// Kalshi cannot participate as it is off-chain. Relevant when moving to live execution.

mod api;
mod audit;
mod orderbook;
mod report;
mod scorer;
mod state;
mod types;
mod ws;

use chrono::Local;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration, Instant};
use types::{FeeRates, MarketMapping, MarketState, OpportunityWindow, SharedBookState};

fn log_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("[{}] WARN  SYSTEM fatal error={}", log_time(), e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let audit_mode = std::env::args().any(|a| a == "--audit");

    if audit_mode {
        let client = Client::new();
        let markets = api::get_tracked_markets();
        eprintln!(
            "[{}] INFO  AUDIT fetching books for {} markets...",
            log_time(),
            markets.len(),
        );
        let polymarket_fee = api::fetch_polymarket_fee_rate(&client)
            .await
            .unwrap_or(0.0);
        let fees = types::FeeRates {
            kalshi_fee: 0.0,
            polymarket_fee,
        };
        eprintln!(
            "[{}] INFO  AUDIT fees kalshi={:.1}% polymarket={:.1}%",
            log_time(),
            fees.kalshi_fee * 100.0,
            fees.polymarket_fee * 100.0,
        );
        audit::run_audit(&markets, &fees).await;
        return Ok(());
    }

    let client = Client::new();
    let start_time = Instant::now();

    // 1. Load persisted state
    let app_state = state::load_state();
    eprintln!(
        "[{}] INFO  SYSTEM state_loaded trades={} executability_entries={}",
        log_time(),
        app_state.trades.len(),
        app_state.executability.len(),
    );

    // 2. Load curated market list (hardcoded — no network fetch needed)
    let matched_markets = api::get_tracked_markets();
    eprintln!(
        "[{}] INFO  SYSTEM markets_loaded count={} source=hardcoded",
        log_time(),
        matched_markets.len(),
    );

    if matched_markets.is_empty() {
        eprintln!(
            "[{}] WARN  SYSTEM no_matched_markets trades_on_record={}",
            log_time(),
            app_state.trades.len(),
        );
        return Ok(());
    }

    // 3. Build mappings and fetch resolution dates
    let mut mappings: Vec<MarketMapping> = Vec::new();
    let app_state = Arc::new(Mutex::new(app_state));

    for market in &matched_markets {
        let kalshi_ticker = match &market.kalshi_ticker {
            Some(t) => t.clone(),
            None => continue,
        };
        let polymarket_token = match &market.polymarket_token {
            Some(t) => t.clone(),
            None => continue,
        };

        // Cache resolution date if not already cached
        {
            let mut app = app_state.lock().await;
            if !app.markets.contains_key(&market.ticker) {
                let res_date = api::fetch_resolution_date(
                    &client,
                    Some(&kalshi_ticker),
                    Some(&polymarket_token),
                )
                .await
                .unwrap_or(None);

                app.markets.insert(
                    market.ticker.clone(),
                    MarketState {
                        resolution_date: res_date,
                    },
                );
            }
        }

        mappings.push(MarketMapping {
            bwr_ticker: market.ticker.clone(),
            kalshi_ticker,
            polymarket_token,
        });
    }

    {
        let app = app_state.lock().await;
        state::save_state(&app)?;
    }

    if mappings.is_empty() {
        eprintln!("[{}] WARN  SYSTEM no_valid_mappings", log_time());
        return Ok(());
    }

    eprintln!(
        "[{}] INFO  SYSTEM tracking count={} markets",
        log_time(),
        mappings.len(),
    );
    for m in &mappings {
        eprintln!(
            "[{}] INFO  {} mapped kalshi={} polymarket={}",
            log_time(),
            m.bwr_ticker,
            m.kalshi_ticker,
            m.polymarket_token,
        );
    }

    // 4. Fetch fee rates
    let polymarket_fee = api::fetch_polymarket_fee_rate(&client)
        .await
        .unwrap_or(0.0);
    let fees = FeeRates {
        kalshi_fee: 0.0, // 0% as of 2026
        polymarket_fee,
    };
    eprintln!(
        "[{}] INFO  SYSTEM fees kalshi={:.1}% polymarket={:.1}%",
        log_time(),
        fees.kalshi_fee * 100.0,
        fees.polymarket_fee * 100.0,
    );

    // 5. Measure execution overhead
    let overhead = api::measure_execution_overhead(&client).await;
    eprintln!(
        "[{}] INFO  SYSTEM execution_floor_ms={} polygon={}ms network_kalshi={}ms network_pm={}ms vpn={}ms",
        log_time(),
        overhead.total_floor_ms(),
        overhead.polygon_settlement_ms,
        overhead.kalshi_network_ms,
        overhead.polymarket_network_ms,
        overhead.vpn_overhead_ms,
    );
    if overhead.vpn_overhead_ms == 0 {
        eprintln!(
            "[{}] WARN  SYSTEM vpn_overhead=0ms — VPN_OVERHEAD_MS not set. Live Polymarket execution requires a non-US VPN.",
            log_time(),
        );
    } else {
        eprintln!(
            "[{}] INFO  SYSTEM vpn_overhead={}ms (set VPN_OVERHEAD_MS to update)",
            log_time(),
            overhead.vpn_overhead_ms,
        );
    }

    // 6. Set up shared state
    let shared_state = Arc::new(RwLock::new(SharedBookState::new()));
    let opportunity_log: Arc<Mutex<Vec<OpportunityWindow>>> = Arc::new(Mutex::new(Vec::new()));

    // 7. Read Kalshi credentials from environment
    let kalshi_email = std::env::var("KALSHI_EMAIL").unwrap_or_default();
    let kalshi_password = std::env::var("KALSHI_PASSWORD").unwrap_or_default();

    // 8. Spawn WebSocket tasks
    let kalshi_state = shared_state.clone();
    let kalshi_log = opportunity_log.clone();
    let kalshi_app = app_state.clone();
    let kalshi_fees = fees.clone();
    let kalshi_mappings = mappings.clone();
    let kalshi_overhead = overhead.clone();
    let kalshi_handle = tokio::spawn(async move {
        if kalshi_email.is_empty() || kalshi_password.is_empty() {
            eprintln!(
                "[{}] WARN  SYSTEM kalshi_ws_skipped reason=no_credentials",
                Local::now().format("%H:%M:%S"),
            );
            loop {
                sleep(Duration::from_secs(3600)).await;
            }
        }
        ws::kalshi_ws_task(
            kalshi_mappings,
            kalshi_state,
            kalshi_log,
            kalshi_app,
            kalshi_fees,
            kalshi_overhead,
            kalshi_email,
            kalshi_password,
        )
        .await;
    });

    let pm_state = shared_state.clone();
    let pm_log = opportunity_log.clone();
    let pm_app = app_state.clone();
    let pm_fees = fees.clone();
    let pm_mappings = mappings.clone();
    let pm_overhead = overhead.clone();
    let pm_handle = tokio::spawn(async move {
        ws::polymarket_ws_task(pm_mappings, pm_state, pm_log, pm_app, pm_fees, pm_overhead).await;
    });

    // 9. Spawn periodic stale-book checker, report writer, and heartbeat
    let report_state = shared_state.clone();
    let report_log = opportunity_log.clone();
    let report_app = app_state.clone();
    let report_overhead = overhead.clone();
    tokio::spawn(async move {
        let mut tick = 0u64;
        loop {
            sleep(Duration::from_secs(30)).await;
            tick += 1;

            // Check for stale books
            ws::check_stale_books(&report_state).await;

            // Write report with current opportunity windows and trade history
            let opportunities = report_log.lock().await;
            let app = report_app.lock().await;
            if let Err(e) = report::write_report(
                &app.executability,
                &report_overhead,
                &opportunities,
                &app.trades,
            ) {
                eprintln!(
                    "[{}] WARN  SYSTEM report_write_failed error={}",
                    Local::now().format("%H:%M:%S"),
                    e,
                );
            }

            // Heartbeat every 60s (every 2 ticks since tick interval is 30s)
            if tick % 2 == 0 {
                let open_windows = opportunities
                    .iter()
                    .filter(|w| w.closed_at_ms.is_none())
                    .count();
                let uptime_secs = start_time.elapsed().as_secs();
                let uptime_str = if uptime_secs < 60 {
                    format!("{}s", uptime_secs)
                } else if uptime_secs < 3600 {
                    format!("{}m{}s", uptime_secs / 60, uptime_secs % 60)
                } else {
                    format!(
                        "{}h{}m",
                        uptime_secs / 3600,
                        (uptime_secs % 3600) / 60,
                    )
                };
                eprintln!(
                    "[{}] INFO  SYSTEM heartbeat open_windows={} trades_total={} uptime={}",
                    Local::now().format("%H:%M:%S"),
                    open_windows,
                    app.trades.len(),
                    uptime_str,
                );
            }
        }
    });

    eprintln!(
        "[{}] INFO  SYSTEM started markets={} trades_on_record={}",
        log_time(),
        mappings.len(),
        {
            let app = app_state.lock().await;
            app.trades.len()
        },
    );

    // Wait for either WS task to complete (they shouldn't under normal operation)
    tokio::select! {
        _ = kalshi_handle => {
            eprintln!("[{}] WARN  SYSTEM kalshi_task_exited", log_time());
        }
        _ = pm_handle => {
            eprintln!("[{}] WARN  SYSTEM polymarket_task_exited", log_time());
        }
    }

    // Save final state before exit
    {
        let opportunities = opportunity_log.lock().await;
        let app = app_state.lock().await;
        let _ = report::write_report(&app.executability, &overhead, &opportunities, &app.trades);
        if let Err(e) = state::save_state(&app) {
            eprintln!(
                "[{}] WARN  SYSTEM final_save_failed error={}",
                log_time(),
                e,
            );
        }
    }

    Ok(())
}
