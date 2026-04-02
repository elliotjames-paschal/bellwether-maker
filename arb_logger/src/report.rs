use crate::types::{ExecutionOverhead, MarketExecutability, OpportunityWindow, TradeRecord};

const REPORT_FILE: &str = "report.md";

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

    // Section 2: Opportunity windows from this session
    out.push_str("## Opportunity Windows (This Session)\n\n");
    if opportunities.is_empty() {
        out.push_str("_No opportunity windows detected this session._\n\n");
    } else {
        out.push_str(
            "| Ticker | Duration (ms) | Peak NEV | Peak Spread | Updates | Status |\n",
        );
        out.push_str(
            "|--------|---------------|----------|-------------|---------|--------|\n",
        );
        for w in opportunities {
            let duration = match w.duration_ms {
                Some(d) => d.to_string(),
                None => "open".to_string(),
            };
            let status = if w.closed_at_ms.is_some() {
                "CLOSED"
            } else {
                "OPEN"
            };
            out.push_str(&format!(
                "| {} | {} | ${:.4} | ${:.4} | {} | {} |\n",
                w.ticker, duration, w.peak_nev, w.peak_spread, w.update_count, status,
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
