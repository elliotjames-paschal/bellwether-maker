use crate::types::{ExecutionOverhead, MarketExecutability, OpportunityWindow, TradeRecord};
use chrono::{TimeZone, Utc};

const REPORT_FILE: &str = "report.md";

fn format_epoch_ms(ms: u64) -> String {
    match Utc.timestamp_millis_opt(ms as i64) {
        chrono::LocalResult::Single(dt) => dt.format("%H:%M:%S").to_string(),
        _ => format!("{}", ms),
    }
}

fn format_duration_human(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{}ms", ms as u64)
    } else if ms < 60_000.0 {
        format!("{:.1}s", ms / 1000.0)
    } else {
        let mins = (ms / 60_000.0).floor() as u64;
        let secs = ((ms % 60_000.0) / 1000.0).floor() as u64;
        format!("{}m {}s", mins, secs)
    }
}

/// Write report.md with executability analysis, opportunity windows, and trade log.
pub fn write_report(
    executability: &[MarketExecutability],
    overhead: &ExecutionOverhead,
    opportunities: &[OpportunityWindow],
    all_trades: &[TradeRecord],
) -> Result<(), String> {
    let mut out = String::new();

    out.push_str("# Arb Logger Report\n\n");

    // Section 1: Executability Analysis
    out.push_str("## Executability Analysis\n\n");
    out.push_str("Execution floor breakdown:\n");
    out.push_str(&format!(
        "  Polygon settlement:      {}ms\n",
        overhead.polygon_settlement_ms,
    ));
    out.push_str(&format!(
        "  Kalshi network:          {}ms\n",
        overhead.kalshi_network_ms,
    ));
    out.push_str(&format!(
        "  Polymarket network:      {}ms\n",
        overhead.polymarket_network_ms,
    ));
    out.push_str(&format!(
        "  Order construction:      ~{}ms\n",
        overhead.kalshi_construction_ms + overhead.polymarket_construction_ms,
    ));
    out.push_str(&format!(
        "  VPN overhead:            {}ms\n",
        overhead.vpn_overhead_ms,
    ));
    out.push_str(&format!(
        "  Total floor:             {}ms\n\n",
        overhead.total_floor_ms(),
    ));
    if overhead.vpn_overhead_ms == 0 {
        out.push_str("_Note: VPN overhead is 0ms. Set VPN\\_OVERHEAD\\_MS env var to your measured value before interpreting executability scores._\n\n");
    }

    if executability.is_empty() {
        out.push_str("_No executability data yet — waiting for opportunity windows to close._\n\n");
    } else {
        out.push_str(
            "| Market | Windows | Avg Duration | P95 Duration | Floor | Executable | Confidence |\n",
        );
        out.push_str(
            "|--------|---------|--------------|--------------|-------|------------|------------|\n",
        );
        for e in executability {
            let exec_icon = if e.executable { "Y" } else { "N" };
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                e.ticker,
                e.window_count,
                format_duration_human(e.avg_duration_ms),
                format_duration_human(e.p95_duration_ms),
                format_duration_human(e.execution_floor_ms),
                exec_icon,
                e.confidence,
            ));
        }
        out.push('\n');
    }

    // Section 2: Live open positions sorted by current NEV
    let mut open: Vec<&OpportunityWindow> = opportunities
        .iter()
        .filter(|w| w.closed_at_ms.is_none())
        .collect();
    open.sort_by(|a, b| b.current_nev.partial_cmp(&a.current_nev).unwrap_or(std::cmp::Ordering::Equal));

    let quality_count = open.iter().filter(|w| w.current_nev >= 0.03 && w.current_depth >= 200.0).count();
    let total_quality_profit: f64 = open.iter()
        .filter(|w| w.current_nev >= 0.03 && w.current_depth >= 200.0)
        .map(|w| w.current_spread)
        .sum();
    let total_quality_capital: f64 = open.iter()
        .filter(|w| w.current_nev >= 0.03 && w.current_depth >= 200.0)
        .map(|w| w.current_depth)
        .sum();

    out.push_str("## Live Arb Opportunities\n\n");
    out.push_str(&format!(
        "**{} quality arbs** (NEV >= 3¢, depth >= $200) | Profit: **${:.2}** | Capital: **${:.2}**\n\n",
        quality_count, total_quality_profit, total_quality_capital,
    ));

    if open.is_empty() {
        out.push_str("_No open opportunities._\n\n");
    } else {
        out.push_str(
            "| Ticker | NEV | Spread | Entry Cost | Full Std Cost | Shares | Direction | Updates |\n",
        );
        out.push_str(
            "|--------|-----|--------|-----------|---------------|--------|-----------|--------|\n",
        );
        for w in &open {
            let (full_std, shares) = match &w.round_trip {
                Some(rd) => (
                    format!("${:.0}", rd.cost_to_fully_standardize),
                    format!("{:.0}", w.best_round_trip.as_ref().map(|rt| rt.shares_filled).unwrap_or(0.0)),
                ),
                None => ("--".to_string(), "--".to_string()),
            };
            out.push_str(&format!(
                "| {} | {:.1}\u{00a2} | ${:.2} | ${:.0} | {} | {} | {} | {} |\n",
                w.ticker, w.current_nev * 100.0, w.current_spread, w.current_depth, full_std, shares, w.current_direction, w.update_count,
            ));
        }
        out.push('\n');
    }

    // Section 2b: Closed windows — full detail
    let mut closed: Vec<&OpportunityWindow> = opportunities
        .iter()
        .filter(|w| w.closed_at_ms.is_some())
        .collect();
    closed.sort_by(|a, b| b.peak_nev.partial_cmp(&a.peak_nev).unwrap_or(std::cmp::Ordering::Equal));

    if !closed.is_empty() {
        out.push_str(&format!("## Closed Windows ({})\n\n", closed.len()));
        out.push_str(
            "| Ticker | Opened (UTC) | Duration | Peak NEV | Profit | Entry Cost | Full Std | Shares | Direction |\n",
        );
        out.push_str(
            "|--------|-------------|----------|----------|--------|-----------|---------|--------|----------|\n",
        );
        for w in &closed {
            let duration = format_duration_human(w.duration_ms.unwrap_or(0) as f64);
            let opened = format_epoch_ms(w.opened_at_ms);
            let (profit, entry_cost, full_std, shares, direction) = match &w.round_trip {
                Some(rd) => (
                    format!("${:.2}", rd.net_profit),
                    format!("${:.0}", rd.cost_to_profitably_standardize),
                    format!("${:.0}", rd.cost_to_fully_standardize),
                    format!("{:.0}", w.best_round_trip.as_ref().map(|rt| rt.shares_filled).unwrap_or(0.0)),
                    rd.direction.clone(),
                ),
                None => ("--".into(), "--".into(), "--".into(), "--".into(), "--".into()),
            };
            out.push_str(&format!(
                "| {} | {} | {} | {:.1}\u{00a2} | {} | {} | {} | {} | {} |\n",
                w.ticker, opened, duration, w.peak_nev * 100.0, profit, entry_cost, full_std, shares, direction,
            ));
        }
        out.push('\n');
    }

    // Section 3: All historical trades
    out.push_str("## Historical Trade Log\n\n");
    if all_trades.is_empty() {
        out.push_str("_No trades on record._\n\n");
    } else {
        let mut sorted: Vec<&TradeRecord> = all_trades.iter().collect();
        sorted.sort_by(|a, b| b.logged_at.cmp(&a.logged_at));

        out.push_str(
            "| Ticker | Logged | Days Left | Net Profit | Ann. Return | Exit Type |\n",
        );
        out.push_str(
            "|--------|--------|-----------|------------|-------------|-----------|\n",
        );
        for t in &sorted {
            let days = match t.days_left {
                Some(d) => d.to_string(),
                None => "N/A".to_string(),
            };
            let ann = match t.ann_return {
                Some(r) => format!("{:.1}%", r * 100.0),
                None => "N/A".to_string(),
            };
            let logged = if t.logged_at.len() >= 19 {
                &t.logged_at[..19]
            } else {
                &t.logged_at
            };
            out.push_str(&format!(
                "| {} | {} | {} | ${:.4} | {} | {} |\n",
                t.ticker, logged, days, t.net_profit, ann, t.exit_type,
            ));
        }
        out.push('\n');
    }

    std::fs::write(REPORT_FILE, &out)
        .map_err(|e| format!("Failed to write {}: {}", REPORT_FILE, e))?;

    Ok(())
}
