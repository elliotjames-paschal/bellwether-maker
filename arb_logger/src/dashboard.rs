use crate::simulator;
use crate::types::SimulatorState;
use chrono::{DateTime, Utc};

const DASHBOARD_FILE: &str = "../dashboard.md";

pub fn write_dashboard(simulator: &SimulatorState) -> Result<(), String> {
    let mut out = String::new();
    let now = Utc::now();

    let projected = simulator::project_total_return(simulator);
    let projected_peak = simulator::project_peak_capital(simulator);

    // ---------------------------------------------------------------------------
    // Header
    // ---------------------------------------------------------------------------
    out.push_str("# Bellwether Maker — Paper Trading Dashboard\n\n");
    out.push_str(&format!(
        "_Last updated: {} UTC_\n\n",
        now.format("%Y-%m-%d %H:%M:%S"),
    ));

    // Session info
    let session_hours = session_duration_hours(&simulator.session_started_at);
    let unique_tickers: std::collections::HashSet<&str> = simulator
        .positions
        .iter()
        .map(|p| p.ticker.as_str())
        .collect();
    out.push_str(&format!(
        "**Session**: {} | **Markets with positions**: {} | **Total paper trades**: {}\n\n",
        format_duration_hours(session_hours),
        unique_tickers.len(),
        simulator.positions.len(),
    ));

    // ---------------------------------------------------------------------------
    // P&L Summary
    // ---------------------------------------------------------------------------
    out.push_str("## P&L Summary\n\n");

    out.push_str("| Metric | Value |\n");
    out.push_str("|--------|-------|\n");
    out.push_str(&format!(
        "| Realized arb profit | **${:.2}** |\n",
        projected.realized_arb,
    ));
    out.push_str(&format!(
        "| Yield on positions (to resolution) | **${:.2}** |\n",
        projected.yield_on_positions,
    ));
    out.push_str(&format!(
        "| **Realized + yield** | **${:.2}** |\n",
        projected.realized_arb + projected.yield_on_positions,
    ));
    out.push_str(&format!(
        "| Projected future arb (sqrt-discounted) | ${:.2} |\n",
        projected.projected_arb,
    ));
    out.push_str(&format!(
        "| Projected future yield | ${:.2} |\n",
        projected.projected_yield,
    ));
    out.push_str(&format!(
        "| **Projected total return** | **${:.2}** |\n",
        projected.total,
    ));

    out.push('\n');

    // ---------------------------------------------------------------------------
    // Capital Requirements
    // ---------------------------------------------------------------------------
    out.push_str("## Capital Requirements\n\n");

    out.push_str("| Metric | Value |\n");
    out.push_str("|--------|-------|\n");
    out.push_str(&format!(
        "| Current capital deployed | ${:.2} |\n",
        simulator.current_capital,
    ));
    out.push_str(&format!(
        "| Peak capital observed | ${:.2} |\n",
        simulator.peak_capital,
    ));
    out.push_str(&format!(
        "| Projected peak capital (incl. re-entries) | **${:.2}** |\n",
        projected_peak,
    ));

    if simulator.current_capital > 0.0 {
        let roi = projected.total / simulator.current_capital * 100.0;
        out.push_str(&format!(
            "| Projected ROI on current capital | {:.1}% |\n",
            roi,
        ));
    }
    if projected_peak > 0.0 {
        let roi_peak = projected.total / projected_peak * 100.0;
        out.push_str(&format!(
            "| Projected ROI on projected peak capital | {:.1}% |\n",
            roi_peak,
        ));
    }

    out.push('\n');

    // ---------------------------------------------------------------------------
    // Platform Yield Rates
    // ---------------------------------------------------------------------------
    out.push_str("## Platform Yield Rates\n\n");
    out.push_str(&format!(
        "| Platform | APY | Source |\n|----------|-----|--------|\n| Kalshi | {:.1}% | Interest on cash + positions |\n| Polymarket | {:.1}% | Position yield on eligible markets |\n\n",
        simulator.kalshi_apy * 100.0,
        simulator.polymarket_apy * 100.0,
    ));
    out.push_str("_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._\n\n");

    // ---------------------------------------------------------------------------
    // Per-Market Positions
    // ---------------------------------------------------------------------------
    out.push_str("## Open Positions\n\n");

    if simulator.positions.is_empty() {
        out.push_str("_No paper positions yet. Waiting for arb opportunities above threshold._\n\n");
    } else {
        out.push_str(
            "| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |\n",
        );
        out.push_str(
            "|--------|---------|---------|------------|-------------|-----------|----------|\n",
        );

        // Aggregate by ticker
        let mut by_ticker: std::collections::BTreeMap<String, TickerAgg> =
            std::collections::BTreeMap::new();
        for pos in &simulator.positions {
            let agg = by_ticker
                .entry(pos.ticker.clone())
                .or_insert_with(|| TickerAgg {
                    entries: 0,
                    capital: 0.0,
                    arb_profit: 0.0,
                    yield_est: 0.0,
                    days_left: pos.days_to_resolution,
                    direction: pos.direction.clone(),
                });
            agg.entries += 1;
            agg.capital += pos.entry_cost;
            agg.arb_profit += pos.net_profit;

            // Yield estimate for this position
            let days = pos.days_to_resolution.unwrap_or(0).max(0) as f64;
            let years = days / 365.0;
            let entry_apy = if pos.entry_platform == "kalshi" {
                simulator.kalshi_apy
            } else {
                simulator.polymarket_apy
            };
            let exit_apy = if pos.exit_platform == "kalshi" {
                simulator.kalshi_apy
            } else {
                simulator.polymarket_apy
            };
            agg.yield_est += pos.entry_cost * entry_apy * years;
            agg.yield_est += pos.exit_proceeds * exit_apy * years;
        }

        for (ticker, agg) in &by_ticker {
            let days_str = agg
                .days_left
                .map(|d| format!("{}", d))
                .unwrap_or_else(|| "N/A".into());
            let short_dir = shorten_direction(&agg.direction);
            out.push_str(&format!(
                "| {} | {} | ${:.2} | ${:.2} | ${:.2} | {} | {} |\n",
                short_ticker(ticker),
                agg.entries,
                agg.capital,
                agg.arb_profit,
                agg.yield_est,
                days_str,
                short_dir,
            ));
        }
        out.push('\n');
    }

    // ---------------------------------------------------------------------------
    // Re-entry Analysis
    // ---------------------------------------------------------------------------
    out.push_str("## Re-entry Analysis\n\n");
    out.push_str("_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. ");
    out.push_str("This assumes arb frequency diminishes as markets approach resolution._\n\n");

    if simulator.reentry_stats.is_empty() {
        out.push_str("_No re-entry data yet._\n\n");
    } else {
        out.push_str(
            "| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |\n",
        );
        out.push_str(
            "|--------|----------|------------|----------|-----------|------------|------------------|\n",
        );

        let mut stats: Vec<_> = simulator.reentry_stats.values().collect();
        stats.sort_by(|a, b| b.entry_count.cmp(&a.entry_count));

        for s in &stats {
            let days_left = simulator
                .positions
                .iter()
                .find(|p| p.ticker == s.ticker)
                .and_then(|p| p.days_to_resolution);

            let projected_entries = simulator::project_reentries(s, days_left);
            let rate_per_day = if s.observation_hours > 0.0 {
                s.entry_count as f64 / (s.observation_hours / 24.0)
            } else {
                0.0
            };
            let projected_profit = projected_entries * s.avg_profit_per_entry;

            out.push_str(&format!(
                "| {} | {} | {:.1}h | {:.1}/d | {:.0} | ${:.2} | ${:.2} |\n",
                short_ticker(&s.ticker),
                s.entry_count,
                s.observation_hours,
                rate_per_day,
                projected_entries,
                s.avg_profit_per_entry,
                projected_profit,
            ));
        }
        out.push('\n');
    }

    // ---------------------------------------------------------------------------
    // Recent Trades
    // ---------------------------------------------------------------------------
    out.push_str("## Recent Paper Trades (last 20)\n\n");

    let recent: Vec<_> = simulator
        .positions
        .iter()
        .rev()
        .take(20)
        .collect();

    if recent.is_empty() {
        out.push_str("_No trades yet._\n\n");
    } else {
        out.push_str(
            "| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |\n",
        );
        out.push_str(
            "|------------|--------|--------|---------|--------|-----|----------|\n",
        );
        for pos in &recent {
            let time = if pos.opened_at.len() >= 19 {
                &pos.opened_at[11..19]
            } else {
                &pos.opened_at
            };
            out.push_str(&format!(
                "| {} | {} | {:.0} | ${:.2} | ${:.2} | {:.1}¢ | {} |\n",
                time,
                short_ticker(&pos.ticker),
                pos.shares,
                pos.entry_cost,
                pos.net_profit,
                pos.nev_per_share * 100.0,
                shorten_direction(&pos.direction),
            ));
        }
        out.push('\n');
    }

    // ---------------------------------------------------------------------------
    // Methodology note
    // ---------------------------------------------------------------------------
    out.push_str("---\n\n");
    out.push_str("### Methodology\n\n");
    out.push_str("- **Paper trades** execute when cross-platform spread exceeds 3¢ NEV with $200+ depth\n");
    out.push_str("- **Positions are held to market resolution** — no early exits\n");
    out.push_str(&format!(
        "- **Yield**: Kalshi {:.1}% APY on positions + cash; Polymarket {:.1}% APY on eligible markets\n",
        simulator.kalshi_apy * 100.0,
        simulator.polymarket_apy * 100.0,
    ));
    out.push_str("- **Re-entry projection**: `observed_count × √(days_remaining / days_observed)` — conservative sqrt discount\n");
    out.push_str("- **Capital projection**: peak observed + (projected re-entries × avg capital per entry)\n");
    out.push_str("- **Understated profits**: real execution would close spreads, allowing them to reopen for additional trades\n");

    std::fs::write(DASHBOARD_FILE, &out)
        .map_err(|e| format!("Failed to write {}: {}", DASHBOARD_FILE, e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct TickerAgg {
    entries: u32,
    capital: f64,
    arb_profit: f64,
    yield_est: f64,
    days_left: Option<i64>,
    direction: String,
}

fn session_duration_hours(started_at: &str) -> f64 {
    if let Ok(dt) = DateTime::parse_from_rfc3339(started_at) {
        let elapsed = Utc::now().signed_duration_since(dt);
        elapsed.num_minutes() as f64 / 60.0
    } else {
        0.0
    }
}

fn format_duration_hours(hours: f64) -> String {
    if hours < 1.0 {
        format!("{:.0}m", hours * 60.0)
    } else if hours < 24.0 {
        format!("{:.1}h", hours)
    } else {
        let days = (hours / 24.0).floor() as u64;
        let h = (hours % 24.0).floor() as u64;
        format!("{}d {}h", days, h)
    }
}

fn short_ticker(ticker: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if parts.len() >= 5 {
        if let Some(idx) = parts.iter().position(|p| {
            p.starts_with("HOUSE") || p.starts_with("GOV") || p.starts_with("SENATE")
        }) {
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

fn shorten_direction(dir: &str) -> &str {
    if dir.contains("POLYMARKET") && dir.contains("KALSHI") {
        if dir.starts_with("BUY_YES_POLYMARKET") {
            "PM→K"
        } else {
            "K→PM"
        }
    } else {
        dir
    }
}
