use crate::types::{
    FeeRates, MarketReentryStats, PaperPosition, PlatformBook, SimulatorState,
};
use crate::orderbook;
use crate::types::LegDetail;
use chrono::{Timelike, Utc};
use ordered_float::OrderedFloat;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Minimum NEV per share to trigger a paper trade.
const MIN_NEV_TRIGGER: f64 = 0.03; // 3¢
/// Minimum executable depth (entry cost) to trigger.
const MIN_DEPTH_TRIGGER: f64 = 200.0; // $200

// ---------------------------------------------------------------------------
// Core: check book update and maybe open a paper position
// ---------------------------------------------------------------------------

/// Called on every book update. If there's a profitable arb above threshold
/// and we haven't already entered on this opportunity window, simulate a trade.
///
/// Returns true if a new position was opened.
pub fn on_book_update(
    bwr_ticker: &str,
    pm_token: &str,
    kalshi_book: &PlatformBook,
    poly_book: &PlatformBook,
    fees: &FeeRates,
    resolution_date: Option<&str>,
    days_left: Option<i64>,
    simulator: &mut SimulatorState,
    // Whether there's currently an open opportunity window for this ticker
    // (to avoid double-entering on the same window)
    has_open_window: bool,
    // How long the opportunity window has been open (0 for new windows)
    window_age_ms: u64,
) -> bool {
    // Only enter when a new window opens (not on every update within a window)
    // We track this by checking if the last entry for this ticker was during
    // a currently-open window. The caller tells us via has_open_window.
    // If the window is already open and we already have a position from this window,
    // skip. We use a simple heuristic: if the most recent position for this ticker
    // was opened less than 5 seconds ago, skip (same window).
    if has_open_window {
        if let Some(last) = simulator.positions.iter().rev().find(|p| p.ticker == bwr_ticker) {
            if let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(&last.opened_at) {
                let elapsed = Utc::now().signed_duration_since(last_dt).num_seconds();
                if elapsed < 5 {
                    return false; // same window, already entered
                }
            }
        }
        // Window was already open before we entered — this is a continuation, skip
        return false;
    }

    // Simulate round trip
    let result = orderbook::simulate_round_trip_btree(
        kalshi_book,
        poly_book,
        fees.kalshi_fee,
        fees.pm_rate(pm_token),
    );

    let (rt, detail) = match result {
        Some(r) => r,
        None => return false,
    };

    let nev = detail.nev_per_share;
    let depth = rt.total_entry_cost;

    if nev < MIN_NEV_TRIGGER || depth < MIN_DEPTH_TRIGGER {
        return false;
    }

    // Compute per-platform internal spreads
    let kalshi_internal = internal_spread(kalshi_book);
    let pm_internal = internal_spread(poly_book);

    // Compute per-platform depth at profitable prices
    let kalshi_depth = platform_depth(&detail.entry_leg, &detail.exit_leg, "kalshi");
    let pm_depth = platform_depth(&detail.entry_leg, &detail.exit_leg, "polymarket");

    // Open a paper position
    let now_dt = Utc::now();
    let now = now_dt.to_rfc3339();
    let hour_utc = now_dt.hour();
    let position = PaperPosition {
        ticker: bwr_ticker.to_string(),
        opened_at: now.clone(),
        hour_utc,
        direction: detail.direction.clone(),
        entry_platform: detail.entry_leg.platform.clone(),
        exit_platform: detail.exit_leg.platform.clone(),
        shares: rt.shares_filled,
        entry_cost: rt.total_entry_cost,
        exit_proceeds: rt.total_exit_proceeds,
        net_profit: detail.net_profit,
        nev_per_share: nev,
        gross_spread: detail.gross_spread,
        kalshi_internal_spread: kalshi_internal,
        pm_internal_spread: pm_internal,
        kalshi_depth,
        pm_depth,
        window_age_ms,
        resolution_date: resolution_date.map(|s| s.to_string()),
        days_to_resolution: days_left,
    };

    eprintln!(
        "[{}] PAPER {} ENTER shares={:.0} cost=${:.2} profit=${:.2} nev={:.1}¢ dir={}",
        chrono::Local::now().format("%H:%M:%S"),
        bwr_ticker,
        position.shares,
        position.entry_cost,
        position.net_profit,
        nev * 100.0,
        detail.direction,
    );

    simulator.positions.push(position);

    // Update capital tracking
    simulator.current_capital = simulator
        .positions
        .iter()
        .map(|p| p.entry_cost)
        .sum();
    if simulator.current_capital > simulator.peak_capital {
        simulator.peak_capital = simulator.current_capital;
    }

    // Update cumulative profit
    simulator.total_arb_profit = simulator
        .positions
        .iter()
        .map(|p| p.net_profit)
        .sum();

    // Update re-entry stats for this market
    update_reentry_stats(bwr_ticker, &detail.net_profit, &depth, &now, simulator);

    true
}

fn update_reentry_stats(
    ticker: &str,
    profit: &f64,
    capital: &f64,
    now: &str,
    simulator: &mut SimulatorState,
) {
    let stats = simulator
        .reentry_stats
        .entry(ticker.to_string())
        .or_insert_with(|| MarketReentryStats {
            ticker: ticker.to_string(),
            entry_count: 0,
            first_entry_at: now.to_string(),
            last_entry_at: now.to_string(),
            observation_hours: 0.0,
            avg_profit_per_entry: 0.0,
            avg_capital_per_entry: 0.0,
        });

    stats.entry_count += 1;
    stats.last_entry_at = now.to_string();

    // Compute observation window
    if let (Ok(first), Ok(last)) = (
        chrono::DateTime::parse_from_rfc3339(&stats.first_entry_at),
        chrono::DateTime::parse_from_rfc3339(&stats.last_entry_at),
    ) {
        stats.observation_hours = last
            .signed_duration_since(first)
            .num_minutes() as f64
            / 60.0;
    }

    // Running averages
    let n = stats.entry_count as f64;
    stats.avg_profit_per_entry =
        stats.avg_profit_per_entry * ((n - 1.0) / n) + profit / n;
    stats.avg_capital_per_entry =
        stats.avg_capital_per_entry * ((n - 1.0) / n) + capital / n;
}

// ---------------------------------------------------------------------------
// Projections
// ---------------------------------------------------------------------------

/// Project re-entries from now to resolution using sqrt discount.
///
/// Model: observed_rate * sqrt(remaining_days / observed_days)
///
/// Rationale: as markets approach resolution, liquidity drops and prices
/// converge, so arb opportunities diminish. Sqrt gives diminishing marginal
/// returns — defensible and conservative.
pub fn project_reentries(stats: &MarketReentryStats, days_to_resolution: Option<i64>) -> f64 {
    let days_left = match days_to_resolution {
        Some(d) if d > 0 => d as f64,
        _ => return 0.0,
    };

    // Need at least some observation window
    if stats.observation_hours < 1.0 || stats.entry_count < 2 {
        return 0.0;
    }

    let observed_days = stats.observation_hours / 24.0;
    if observed_days < 0.01 {
        return 0.0;
    }

    // Observed rate: entries per day
    let _rate_per_day = stats.entry_count as f64 / observed_days;

    // Project with sqrt discount
    // If we observed R entries/day over D_obs days, project:
    //   R * sqrt(D_remaining / D_observed) * D_observed
    // Simplified: entry_count * sqrt(D_remaining / D_observed)
    let projected = stats.entry_count as f64 * (days_left / observed_days).sqrt();

    projected
}

/// Compute projected peak capital: current peak + projected re-entries * avg capital per entry.
pub fn project_peak_capital(simulator: &SimulatorState) -> f64 {
    let mut projected_additional = 0.0;

    for (ticker, stats) in &simulator.reentry_stats {
        let days_left = simulator
            .positions
            .iter()
            .find(|p| &p.ticker == ticker)
            .and_then(|p| p.days_to_resolution);

        let future_entries = project_reentries(stats, days_left);
        // Each re-entry requires avg_capital but some capital gets released at resolution.
        // Conservative: assume all positions are concurrent (worst case).
        projected_additional += future_entries * stats.avg_capital_per_entry;
    }

    simulator.peak_capital + projected_additional
}

/// Compute total projected profit to resolution.
///
/// Components:
/// 1. Realized arb profit (positions already taken)
/// 2. Projected future arb profit from re-entries (sqrt-discounted)
/// 3. Yield on all capital deployed (current + projected) at platform APY rates
pub fn project_total_return(simulator: &SimulatorState) -> ProjectedReturn {
    let realized_arb = simulator.total_arb_profit;

    // Projected future arb from re-entries
    let mut projected_arb = 0.0;
    for (ticker, stats) in &simulator.reentry_stats {
        let days_left = simulator
            .positions
            .iter()
            .find(|p| &p.ticker == ticker)
            .and_then(|p| p.days_to_resolution);

        let future_entries = project_reentries(stats, days_left);
        projected_arb += future_entries * stats.avg_profit_per_entry;
    }

    // Yield on current positions (held to resolution)
    // Polymarket 4% APY only applies to 13 specific aggregate markets, NOT individual races.
    // Individual race arbs (which are all our positions) earn 0% on Polymarket.
    // Kalshi pays 3.5% on all positions.
    let mut total_yield = 0.0;
    for pos in &simulator.positions {
        let days = pos.days_to_resolution.unwrap_or(0).max(0) as f64;
        let years = days / 365.0;

        // Only Kalshi legs earn yield. Polymarket individual race positions earn 0%.
        let entry_apy = if pos.entry_platform == "kalshi" {
            simulator.kalshi_apy
        } else {
            0.0
        };
        let exit_apy = if pos.exit_platform == "kalshi" {
            simulator.kalshi_apy
        } else {
            0.0
        };

        total_yield += pos.entry_cost * entry_apy * years;
        total_yield += pos.exit_proceeds * exit_apy * years;
    }

    // Projected yield on future positions
    // Only Kalshi leg earns yield, so use kalshi_apy on ~half the capital
    // (roughly one leg is Kalshi, one is Polymarket)
    let mut projected_yield = 0.0;
    for (ticker, stats) in &simulator.reentry_stats {
        let days_left = simulator
            .positions
            .iter()
            .find(|p| &p.ticker == ticker)
            .and_then(|p| p.days_to_resolution);

        let days = days_left.unwrap_or(0).max(0) as f64;
        let future_entries = project_reentries(stats, days_left);
        // Only Kalshi leg earns yield — roughly half the capital per trade
        let avg_hold_years = (days / 2.0) / 365.0;
        projected_yield += future_entries * stats.avg_capital_per_entry * 0.5 * simulator.kalshi_apy * avg_hold_years;
    }

    ProjectedReturn {
        realized_arb,
        projected_arb,
        yield_on_positions: total_yield,
        projected_yield,
        total: realized_arb + projected_arb + total_yield + projected_yield,
    }
}

#[derive(Debug, Clone)]
pub struct ProjectedReturn {
    pub realized_arb: f64,
    pub projected_arb: f64,
    pub yield_on_positions: f64,
    pub projected_yield: f64,
    pub total: f64,
}

// ---------------------------------------------------------------------------
// Helpers for book analysis at entry time
// ---------------------------------------------------------------------------

/// Internal spread (best_ask - best_bid) on a single platform.
fn internal_spread(book: &PlatformBook) -> f64 {
    match (book.bids.keys().next_back(), book.asks.keys().next()) {
        (Some(bid), Some(ask)) => ask.into_inner() - bid.into_inner(),
        _ => f64::NAN,
    }
}

/// Total dollar depth on a given platform's side of the trade.
fn platform_depth(entry_leg: &LegDetail, exit_leg: &LegDetail, platform: &str) -> f64 {
    if entry_leg.platform == platform {
        entry_leg.total_cost
    } else if exit_leg.platform == platform {
        exit_leg.total_cost
    } else {
        0.0
    }
}
