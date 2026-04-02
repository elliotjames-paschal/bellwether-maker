use crate::types::{ExecutionOverhead, MarketExecutability, OpportunityWindow, RoundTrip, TradeRecord};
use chrono::Utc;

/// Minimum net edge per share to consider a trade worth logging.
const MIN_NEV_PER_SHARE: f64 = 0.03;

/// Compute P&L metrics for a round trip and produce a TradeRecord if profitable enough.
pub fn compute_pnl(
    round_trip: &RoundTrip,
    ticker: &str,
    resolution_date: Option<&str>,
    days_left: Option<i64>,
) -> Option<TradeRecord> {
    let gross_profit = round_trip.total_exit_proceeds - round_trip.total_entry_cost;

    // total_entry_cost and total_exit_proceeds are already fee-adjusted
    // (entry: ask * (1 + fee), exit: bid * (1 - fee)) so no additional deduction needed.
    let fees = 0.0;
    let net_profit = gross_profit - fees;

    if round_trip.shares_filled <= 0.0 {
        return None;
    }

    let nev_per_share = net_profit / round_trip.shares_filled;
    if nev_per_share <= MIN_NEV_PER_SHARE {
        return None;
    }

    let avg_entry_price = round_trip.total_entry_cost / round_trip.shares_filled;
    let avg_exit_price = if round_trip.shares_exited > 0.0 {
        round_trip.total_exit_proceeds / round_trip.shares_exited
    } else {
        0.0
    };

    let exit_type =
        if (round_trip.shares_exited - round_trip.shares_filled).abs() < f64::EPSILON {
            "FULL_EXIT".to_string()
        } else {
            "PARTIAL_EXIT".to_string()
        };

    let ann_return = match days_left {
        Some(d) if d > 0 && round_trip.total_entry_cost > 0.0 => {
            Some((net_profit / round_trip.total_entry_cost) * (365.0 / d as f64))
        }
        _ => None,
    };

    Some(TradeRecord {
        ticker: ticker.to_string(),
        logged_at: Utc::now().to_rfc3339(),
        resolution_date: resolution_date.map(|s| s.to_string()),
        days_left,
        direction: round_trip.direction.clone(),
        entry_fills: round_trip.entry_fills.clone(),
        avg_entry_price,
        total_entry_cost: round_trip.total_entry_cost,
        exit_fills: round_trip.exit_fills.clone(),
        avg_exit_price,
        total_exit_proceeds: round_trip.total_exit_proceeds,
        shares_filled: round_trip.shares_filled,
        shares_exited: round_trip.shares_exited,
        exit_type,
        gross_profit,
        fees,
        net_profit,
        nev_per_share,
        ann_return,
    })
}

/// Compute executability analysis for a specific ticker based on all closed
/// opportunity windows observed so far.
pub fn compute_executability(
    ticker: &str,
    closed_windows: &[OpportunityWindow],
    overhead: &ExecutionOverhead,
    avg_message_latency_ms: f64,
) -> MarketExecutability {
    // Collect durations from closed windows for this ticker
    let mut durations: Vec<f64> = closed_windows
        .iter()
        .filter(|w| w.ticker == ticker && w.duration_ms.is_some())
        .map(|w| w.duration_ms.unwrap() as f64)
        .collect();

    let window_count = durations.len() as u32;
    let execution_floor_ms = overhead.total_floor_ms() as f64;

    if durations.is_empty() {
        return MarketExecutability {
            ticker: ticker.to_string(),
            window_count: 0,
            avg_duration_ms: 0.0,
            p50_duration_ms: 0.0,
            p95_duration_ms: 0.0,
            execution_floor_ms,
            executable: false,
            confidence: "INSUFFICIENT".to_string(),
            avg_message_latency_ms,
        };
    }

    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let avg_duration_ms = durations.iter().sum::<f64>() / durations.len() as f64;
    let p50_duration_ms = percentile(&durations, 50.0);
    let p95_duration_ms = percentile(&durations, 95.0);

    let executable = p95_duration_ms > execution_floor_ms;

    let confidence = if window_count >= 20 {
        "HIGH".to_string()
    } else if window_count >= 5 {
        "LOW".to_string()
    } else {
        "INSUFFICIENT".to_string()
    };

    MarketExecutability {
        ticker: ticker.to_string(),
        window_count,
        avg_duration_ms,
        p50_duration_ms,
        p95_duration_ms,
        execution_floor_ms,
        executable,
        confidence,
        avg_message_latency_ms,
    }
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = (pct / 100.0) * (sorted.len() - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let frac = idx - lower as f64;
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}
