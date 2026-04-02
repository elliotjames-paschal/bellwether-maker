use crate::orderbook;
use crate::scorer;
use crate::state;
use crate::types::{
    AppState, ExecutionOverhead, FeeRates, MarketMapping, OpportunityWindow, PlatformBook,
    SharedBookState,
};
use chrono::{Local, Utc};
use futures_util::{SinkExt, StreamExt};
use ordered_float::OrderedFloat;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const KALSHI_WS_URL: &str = "wss://api.elections.kalshi.com/trade-api/v2/ws";
const KALSHI_LOGIN_URL: &str = "https://api.elections.kalshi.com/trade-api/v2/login";
const POLYMARKET_WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

const STALE_BOOK_SECS: u64 = 60;
const MAX_BACKOFF_SECS: u64 = 60;

fn log_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn fmt_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{}s", ms / 1000)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m{}s", mins, secs)
    }
}

// ---------------------------------------------------------------------------
// Kalshi WebSocket task
// ---------------------------------------------------------------------------

pub async fn kalshi_ws_task(
    mappings: Vec<MarketMapping>,
    state: Arc<RwLock<SharedBookState>>,
    opportunity_log: Arc<Mutex<Vec<OpportunityWindow>>>,
    app_state: Arc<Mutex<AppState>>,
    fees: FeeRates,
    overhead: ExecutionOverhead,
    kalshi_email: String,
    kalshi_password: String,
) {
    let kalshi_tickers: Vec<String> = mappings.iter().map(|m| m.kalshi_ticker.clone()).collect();
    let mut backoff_secs = 1u64;
    let mut attempt = 0u64;

    loop {
        attempt += 1;

        match run_kalshi_session(
            &kalshi_tickers,
            &mappings,
            &state,
            &opportunity_log,
            &app_state,
            &fees,
            &overhead,
            &kalshi_email,
            &kalshi_password,
        )
        .await
        {
            Ok(()) => {
                eprintln!(
                    "[{}] WARN  SYSTEM kalshi_ws_disconnected reconnect_in=1s reason=clean_close",
                    log_time(),
                );
                backoff_secs = 1;
                attempt = 0;
            }
            Err(e) => {
                eprintln!(
                    "[{}] WARN  SYSTEM kalshi_ws_disconnected reconnect_in={}s attempt={} error={}",
                    log_time(),
                    backoff_secs,
                    attempt,
                    e,
                );
            }
        }

        sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
    }
}

async fn run_kalshi_session(
    kalshi_tickers: &[String],
    mappings: &[MarketMapping],
    state: &Arc<RwLock<SharedBookState>>,
    opportunity_log: &Arc<Mutex<Vec<OpportunityWindow>>>,
    app_state: &Arc<Mutex<AppState>>,
    fees: &FeeRates,
    overhead: &ExecutionOverhead,
    email: &str,
    password: &str,
) -> Result<(), String> {
    // Authenticate via REST to get JWT
    let client = reqwest::Client::new();
    let login_body = serde_json::json!({
        "email": email,
        "password": password,
    });
    let login_resp: Value = client
        .post(KALSHI_LOGIN_URL)
        .json(&login_body)
        .send()
        .await
        .map_err(|e| format!("Kalshi login request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Kalshi login parse failed: {}", e))?;

    let token = login_resp["token"]
        .as_str()
        .ok_or_else(|| "Kalshi login: missing token".to_string())?;

    // Connect WebSocket with auth header
    let connect_start = std::time::Instant::now();
    let url = format!("{}?token={}", KALSHI_WS_URL, token);
    let (ws_stream, _) = connect_async(&url)
        .await
        .map_err(|e| format!("Kalshi WS connect failed: {}", e))?;
    let connect_ms = connect_start.elapsed().as_millis();

    let (mut write, mut read) = ws_stream.split();
    eprintln!(
        "[{}] INFO  SYSTEM kalshi_ws_connected latency_ms={}",
        log_time(),
        connect_ms,
    );

    // Subscribe to orderbook_delta for all tracked tickers
    let sub_msg = serde_json::json!({
        "id": 1,
        "cmd": "subscribe",
        "params": {
            "channels": ["orderbook_delta"],
            "market_tickers": kalshi_tickers,
        }
    });
    write
        .send(Message::Text(sub_msg.to_string()))
        .await
        .map_err(|e| format!("Kalshi subscribe failed: {}", e))?;

    while let Some(msg_result) = read.next().await {
        let msg = msg_result.map_err(|e| format!("Kalshi WS read error: {}", e))?;

        let text = match msg {
            Message::Text(t) => t,
            Message::Ping(data) => {
                let _ = write.send(Message::Pong(data)).await;
                continue;
            }
            Message::Close(_) => return Ok(()),
            _ => continue,
        };

        let json: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = json["type"].as_str().unwrap_or("");
        let receive_ts_ms = Utc::now().timestamp_millis();

        match msg_type {
            "orderbook_snapshot" => {
                handle_kalshi_snapshot(&json["msg"], state).await;
                if let Some(ticker) = json["msg"]["market_ticker"].as_str() {
                    run_scorer_for_kalshi_ticker(
                        ticker, mappings, state, opportunity_log, app_state, fees, overhead,
                    )
                    .await;
                }
            }
            "orderbook_delta" => {
                let seq = json["seq"].as_u64();
                // Extract exchange timestamp for latency measurement
                let exchange_ts =
                    json["ts"].as_i64().or_else(|| json["msg"]["ts"].as_i64());
                let needs_reconnect =
                    handle_kalshi_delta(&json["msg"], seq, exchange_ts, receive_ts_ms, state).await;
                if needs_reconnect {
                    return Err("Sequence gap detected, reconnecting".to_string());
                }
                if let Some(ticker) = json["msg"]["market_ticker"].as_str() {
                    run_scorer_for_kalshi_ticker(
                        ticker, mappings, state, opportunity_log, app_state, fees, overhead,
                    )
                    .await;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_kalshi_snapshot(msg: &Value, state: &Arc<RwLock<SharedBookState>>) {
    let ticker = match msg["market_ticker"].as_str() {
        Some(t) => t.to_string(),
        None => return,
    };

    let mut book = PlatformBook::new();

    // Parse YES bids (dollar strings)
    if let Some(yes_levels) = msg["yes_dollars_fp"].as_array() {
        for level in yes_levels {
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

    // Parse NO bids -> YES asks (complementary: YES ask = 1.0 - NO bid)
    if let Some(no_levels) = msg["no_dollars_fp"].as_array() {
        for level in no_levels {
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

    book.last_updated = Instant::now();

    let mut s = state.write().await;
    s.kalshi.insert(ticker, book);
}

/// Apply a Kalshi delta. Returns true if a reconnect is needed (sequence gap).
async fn handle_kalshi_delta(
    msg: &Value,
    seq: Option<u64>,
    exchange_ts_ms: Option<i64>,
    receive_ts_ms: i64,
    state: &Arc<RwLock<SharedBookState>>,
) -> bool {
    let ticker = match msg["market_ticker"].as_str() {
        Some(t) => t.to_string(),
        None => return false,
    };

    let mut s = state.write().await;
    let book = s.kalshi.entry(ticker).or_insert_with(PlatformBook::new);

    // Sequence gap detection
    if let Some(new_seq) = seq {
        if let Some(prev_seq) = book.seq {
            if new_seq != prev_seq + 1 {
                eprintln!(
                    "[{}] WARN  SYSTEM kalshi_seq_gap expected={} got={}",
                    log_time(),
                    prev_seq + 1,
                    new_seq,
                );
                return true; // need reconnect
            }
        }
        book.seq = Some(new_seq);
    }

    let price_str = msg["price_dollars"].as_str().unwrap_or("0");
    let price: f64 = price_str.parse().unwrap_or(0.0);
    let delta_str = msg["delta_fp"].as_str().unwrap_or("0");
    let delta: f64 = delta_str.parse().unwrap_or(0.0);
    let side = msg["side"].as_str().unwrap_or("");

    match side {
        "yes" => {
            let key = OrderedFloat(price);
            let current = book.bids.get(&key).copied().unwrap_or(0.0);
            let new_size = current + delta;
            if new_size <= 0.0 {
                book.bids.remove(&key);
            } else {
                book.bids.insert(key, new_size);
            }
        }
        "no" => {
            // NO delta affects YES asks: YES ask price = 1.0 - NO price
            let yes_price = 1.0 - price;
            let key = OrderedFloat(yes_price);
            let current = book.asks.get(&key).copied().unwrap_or(0.0);
            let new_size = current + delta;
            if new_size <= 0.0 {
                book.asks.remove(&key);
            } else {
                book.asks.insert(key, new_size);
            }
        }
        _ => {}
    }

    book.last_updated = Instant::now();

    // Record message latency from exchange timestamp
    if let Some(exch_ts) = exchange_ts_ms {
        let latency = receive_ts_ms - exch_ts;
        book.record_latency(latency);
    }

    false
}

async fn run_scorer_for_kalshi_ticker(
    kalshi_ticker: &str,
    mappings: &[MarketMapping],
    state: &Arc<RwLock<SharedBookState>>,
    opportunity_log: &Arc<Mutex<Vec<OpportunityWindow>>>,
    app_state: &Arc<Mutex<AppState>>,
    fees: &FeeRates,
    overhead: &ExecutionOverhead,
) {
    let mapping = match mappings.iter().find(|m| m.kalshi_ticker == kalshi_ticker) {
        Some(m) => m,
        None => return,
    };

    on_book_update(
        &mapping.bwr_ticker,
        &mapping.kalshi_ticker,
        &mapping.polymarket_token,
        state,
        opportunity_log,
        app_state,
        fees,
        overhead,
    )
    .await;
}

// ---------------------------------------------------------------------------
// Polymarket WebSocket task
// ---------------------------------------------------------------------------

pub async fn polymarket_ws_task(
    mappings: Vec<MarketMapping>,
    state: Arc<RwLock<SharedBookState>>,
    opportunity_log: Arc<Mutex<Vec<OpportunityWindow>>>,
    app_state: Arc<Mutex<AppState>>,
    fees: FeeRates,
    overhead: ExecutionOverhead,
) {
    let token_ids: Vec<String> = mappings
        .iter()
        .map(|m| m.polymarket_token.clone())
        .collect();
    let mut backoff_secs = 1u64;
    let mut attempt = 0u64;

    loop {
        attempt += 1;

        match run_polymarket_session(
            &token_ids,
            &mappings,
            &state,
            &opportunity_log,
            &app_state,
            &fees,
            &overhead,
        )
        .await
        {
            Ok(()) => {
                eprintln!(
                    "[{}] WARN  SYSTEM polymarket_ws_disconnected reconnect_in=1s reason=clean_close",
                    log_time(),
                );
                backoff_secs = 1;
                attempt = 0;
            }
            Err(e) => {
                eprintln!(
                    "[{}] WARN  SYSTEM polymarket_ws_disconnected reconnect_in={}s attempt={} error={}",
                    log_time(),
                    backoff_secs,
                    attempt,
                    e,
                );
            }
        }

        sleep(Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
    }
}

async fn run_polymarket_session(
    token_ids: &[String],
    mappings: &[MarketMapping],
    state: &Arc<RwLock<SharedBookState>>,
    opportunity_log: &Arc<Mutex<Vec<OpportunityWindow>>>,
    app_state: &Arc<Mutex<AppState>>,
    fees: &FeeRates,
    overhead: &ExecutionOverhead,
) -> Result<(), String> {
    let connect_start = std::time::Instant::now();
    let (ws_stream, _) = connect_async(POLYMARKET_WS_URL)
        .await
        .map_err(|e| format!("Polymarket WS connect failed: {}", e))?;
    let connect_ms = connect_start.elapsed().as_millis();

    let (mut write, mut read) = ws_stream.split();
    eprintln!(
        "[{}] INFO  SYSTEM polymarket_ws_connected latency_ms={}",
        log_time(),
        connect_ms,
    );

    // Subscribe to market data for all tracked token IDs
    let sub_msg = serde_json::json!({
        "type": "market",
        "assets_ids": token_ids,
    });
    write
        .send(Message::Text(sub_msg.to_string()))
        .await
        .map_err(|e| format!("Polymarket subscribe failed: {}", e))?;

    while let Some(msg_result) = read.next().await {
        let msg = msg_result.map_err(|e| format!("Polymarket WS read error: {}", e))?;

        match msg {
            Message::Ping(data) => {
                let _ = write.send(Message::Pong(data)).await;
                continue;
            }
            Message::Close(_) => return Ok(()),
            Message::Text(text) => {
                let json: Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let event_type = json["event_type"].as_str().unwrap_or("");
                let receive_ts_ms = Utc::now().timestamp_millis();

                match event_type {
                    "book" => {
                        let asset_id = json["asset_id"].as_str().unwrap_or("").to_string();
                        handle_polymarket_book(&json, &asset_id, state).await;
                        run_scorer_for_pm_token(
                            &asset_id, mappings, state, opportunity_log, app_state, fees, overhead,
                        )
                        .await;
                    }
                    "price_change" => {
                        let asset_id = json["asset_id"].as_str().unwrap_or("").to_string();
                        // Extract exchange timestamp for latency measurement
                        let exchange_ts = json["timestamp"]
                            .as_i64()
                            .or_else(|| {
                                json["timestamp"]
                                    .as_str()
                                    .and_then(|s| s.parse::<i64>().ok())
                            })
                            .map(|ts| {
                                // Auto-detect seconds vs milliseconds
                                if ts < 10_000_000_000 {
                                    ts * 1000
                                } else {
                                    ts
                                }
                            });
                        handle_polymarket_price_change(
                            &json, &asset_id, exchange_ts, receive_ts_ms, state,
                        )
                        .await;
                        run_scorer_for_pm_token(
                            &asset_id, mappings, state, opportunity_log, app_state, fees, overhead,
                        )
                        .await;
                    }
                    _ => {}
                }
            }
            _ => continue,
        }
    }

    Ok(())
}

async fn handle_polymarket_book(
    json: &Value,
    asset_id: &str,
    state: &Arc<RwLock<SharedBookState>>,
) {
    let mut book = PlatformBook::new();

    if let Some(bids) = json["bids"].as_array() {
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

    if let Some(asks) = json["asks"].as_array() {
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

    book.last_updated = Instant::now();

    let mut s = state.write().await;
    s.polymarket.insert(asset_id.to_string(), book);
}

async fn handle_polymarket_price_change(
    json: &Value,
    asset_id: &str,
    exchange_ts_ms: Option<i64>,
    receive_ts_ms: i64,
    state: &Arc<RwLock<SharedBookState>>,
) {
    let changes = match json["changes"].as_array() {
        Some(c) => c,
        None => return,
    };

    let mut s = state.write().await;
    let book = s
        .polymarket
        .entry(asset_id.to_string())
        .or_insert_with(PlatformBook::new);

    for change in changes {
        let price = change["price"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let size = change["size"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let side = change["side"].as_str().unwrap_or("");

        let key = OrderedFloat(price);

        match side {
            "BUY" => {
                if size <= 0.0 {
                    book.bids.remove(&key);
                } else {
                    book.bids.insert(key, size);
                }
            }
            "SELL" => {
                if size <= 0.0 {
                    book.asks.remove(&key);
                } else {
                    book.asks.insert(key, size);
                }
            }
            _ => {}
        }
    }

    book.last_updated = Instant::now();

    // Record message latency from exchange timestamp
    if let Some(exch_ts) = exchange_ts_ms {
        let latency = receive_ts_ms - exch_ts;
        book.record_latency(latency);
    }
}

async fn run_scorer_for_pm_token(
    pm_token: &str,
    mappings: &[MarketMapping],
    state: &Arc<RwLock<SharedBookState>>,
    opportunity_log: &Arc<Mutex<Vec<OpportunityWindow>>>,
    app_state: &Arc<Mutex<AppState>>,
    fees: &FeeRates,
    overhead: &ExecutionOverhead,
) {
    let mapping = match mappings.iter().find(|m| m.polymarket_token == pm_token) {
        Some(m) => m,
        None => return,
    };

    on_book_update(
        &mapping.bwr_ticker,
        &mapping.kalshi_ticker,
        &mapping.polymarket_token,
        state,
        opportunity_log,
        app_state,
        fees,
        overhead,
    )
    .await;
}

// ---------------------------------------------------------------------------
// Opportunity detection -- runs on every book update from either platform
// ---------------------------------------------------------------------------

async fn on_book_update(
    bwr_ticker: &str,
    kalshi_ticker: &str,
    pm_token: &str,
    state: &Arc<RwLock<SharedBookState>>,
    opportunity_log: &Arc<Mutex<Vec<OpportunityWindow>>>,
    app_state: &Arc<Mutex<AppState>>,
    fees: &FeeRates,
    overhead: &ExecutionOverhead,
) {
    let books = state.read().await;
    let kalshi_book = match books.kalshi.get(kalshi_ticker) {
        Some(b) => b,
        None => return,
    };
    let poly_book = match books.polymarket.get(pm_token) {
        Some(b) => b,
        None => return,
    };

    // Check for stale books (silent — check_stale_books handles warnings)
    let now = Instant::now();
    if now.duration_since(kalshi_book.last_updated).as_secs() > STALE_BOOK_SECS {
        return;
    }
    if now.duration_since(poly_book.last_updated).as_secs() > STALE_BOOK_SECS {
        return;
    }

    let result = orderbook::simulate_round_trip_btree(kalshi_book, poly_book, fees);
    let avg_msg_latency = kalshi_book.avg_message_latency_ms.unwrap_or(0.0);
    drop(books); // release read lock before acquiring mutexes

    let mut log = opportunity_log.lock().await;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Find current open window for this ticker
    let open_idx = log
        .iter()
        .position(|w| w.ticker == bwr_ticker && w.closed_at_ms.is_none());

    match result {
        Some((rt, detail)) => {
            let nev = detail.nev_per_share;
            let spread = detail.gross_spread;

            if let Some(idx) = open_idx {
                // Update existing window
                let window = &mut log[idx];
                window.update_count += 1;
                if nev > window.peak_nev {
                    window.peak_nev = nev;
                    window.best_round_trip = Some(rt);
                }
                if spread > window.peak_spread {
                    window.peak_spread = spread;
                }
                if let Some(ref mut rd) = window.round_trip {
                    rd.residual_spread_after_trade = detail.residual_spread_after_trade;
                    rd.fully_standardized = detail.fully_standardized;
                }
            } else {
                // Open new window
                eprintln!(
                    "[{}] OPEN  {} spread={:.1}\u{00a2} nev={:.1}\u{00a2} direction={}",
                    log_time(),
                    bwr_ticker,
                    spread * 100.0,
                    nev * 100.0,
                    detail.direction,
                );
                log.push(OpportunityWindow {
                    ticker: bwr_ticker.to_string(),
                    opened_at_ms: now_ms,
                    closed_at_ms: None,
                    duration_ms: None,
                    peak_nev: nev,
                    peak_spread: spread,
                    update_count: 1,
                    best_round_trip: Some(rt),
                    round_trip: Some(detail),
                });
            }
        }
        None => {
            // Close open window if one exists
            if let Some(idx) = open_idx {
                log[idx].closed_at_ms = Some(now_ms);
                log[idx].duration_ms = Some(now_ms - log[idx].opened_at_ms);

                let duration = log[idx].duration_ms.unwrap_or(0);
                let peak_nev = log[idx].peak_nev;
                let ticker = log[idx].ticker.clone();
                let best_rt = log[idx].best_round_trip.take();
                let fully_std = log[idx]
                    .round_trip
                    .as_ref()
                    .map(|rd| rd.fully_standardized)
                    .unwrap_or(false);

                // Compute executability from all closed windows for this ticker
                let closed_windows: Vec<OpportunityWindow> = log
                    .iter()
                    .filter(|w| w.ticker == bwr_ticker && w.closed_at_ms.is_some())
                    .cloned()
                    .collect();
                let exec = scorer::compute_executability(
                    bwr_ticker,
                    &closed_windows,
                    overhead,
                    avg_msg_latency,
                );

                // Lock app state once for both executability update and trade logging
                let mut app = app_state.lock().await;

                // Update executability for this ticker
                app.executability.retain(|e| e.ticker != bwr_ticker);
                app.executability.push(exec);

                // Score the best round trip and log a trade if it passes threshold
                if let Some(rt) = best_rt {
                    let (resolution_date, days_left) = match app.markets.get(&ticker) {
                        Some(ms) => {
                            let res = ms.resolution_date.as_deref();
                            let days = res.and_then(|d| compute_days_left(d));
                            (res.map(|s| s.to_string()), days)
                        }
                        None => (None, None),
                    };

                    if let Some(trade) = scorer::compute_pnl(
                        &rt,
                        &ticker,
                        resolution_date.as_deref(),
                        days_left,
                    ) {
                        eprintln!(
                            "[{}] CLOSE {} duration={} peak_nev={:.1}\u{00a2} profit=${:.2} fully_standardized={}",
                            log_time(),
                            ticker,
                            fmt_duration(duration),
                            peak_nev * 100.0,
                            trade.net_profit,
                            fully_std,
                        );
                        app.trades.push(trade);

                        // Persist immediately so trades survive crashes
                        if let Err(e) = state::save_state(&app) {
                            eprintln!(
                                "[{}] WARN  SYSTEM state_save_failed error={}",
                                log_time(),
                                e,
                            );
                        }
                    } else {
                        eprintln!(
                            "[{}] CLOSE {} duration={} peak_nev={:.1}\u{00a2} below_threshold skipped",
                            log_time(),
                            ticker,
                            fmt_duration(duration),
                            peak_nev * 100.0,
                        );
                    }
                } else {
                    eprintln!(
                        "[{}] CLOSE {} duration={} peak_nev={:.1}\u{00a2} no_round_trip skipped",
                        log_time(),
                        ticker,
                        fmt_duration(duration),
                        peak_nev * 100.0,
                    );
                }
            }
        }
    }
}

/// Parse a resolution date string into days remaining from now.
fn compute_days_left(date_str: &str) -> Option<i64> {
    // Try ISO 8601 datetime (RFC 3339)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.signed_duration_since(Utc::now()).num_days());
    }
    // Try date-only
    if let Ok(nd) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let today = Utc::now().date_naive();
        return Some((nd - today).num_days());
    }
    None
}

/// Check for stale books across all tracked markets and log warnings.
pub async fn check_stale_books(state: &Arc<RwLock<SharedBookState>>) {
    let books = state.read().await;
    let now = Instant::now();

    for (ticker, book) in &books.kalshi {
        let secs = now.duration_since(book.last_updated).as_secs();
        if secs > STALE_BOOK_SECS {
            eprintln!(
                "[{}] WARN  SYSTEM stale_book platform=kalshi ticker={} seconds_since_update={}",
                log_time(),
                ticker,
                secs,
            );
        }
    }

    for (token, book) in &books.polymarket {
        let secs = now.duration_since(book.last_updated).as_secs();
        if secs > STALE_BOOK_SECS {
            eprintln!(
                "[{}] WARN  SYSTEM stale_book platform=polymarket ticker={} seconds_since_update={}",
                log_time(),
                token,
                secs,
            );
        }
    }
}
