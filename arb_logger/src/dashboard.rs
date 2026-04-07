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
        "| Platform | APY | Applies To |\n|----------|-----|------------|\n| Kalshi | {:.1}% | All positions + cash (min $250 balance) |\n| Polymarket | {:.1}% | **13 specific aggregate markets only** (not individual races) |\n\n",
        simulator.kalshi_apy * 100.0,
        simulator.polymarket_apy * 100.0,
    ));
    out.push_str("_Our individual race arbs earn yield only on the Kalshi leg. Polymarket's 4% yield is limited to broad markets like \"Balance of Power: 2026 Midterms\" and \"Which party wins the House?\", not individual district races._\n\n");

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
    // How each metric is calculated
    // ---------------------------------------------------------------------------
    out.push_str("---\n\n");
    out.push_str("## How Each Metric Is Calculated\n\n");

    out.push_str("### P&L Summary\n\n");
    out.push_str("| Metric | Calculation |\n");
    out.push_str("|--------|-------------|\n");
    out.push_str("| **Realized arb profit** | Sum of `net_profit` across all paper trades. Each trade's profit = `exit_proceeds - entry_cost`, computed by walking both order books and matching shares at each price level until the spread is consumed. |\n");
    out.push_str(&format!(
        "| **Yield on positions** | For each position, only the Kalshi leg earns yield: `kalshi_leg_value × {:.1}% × (days_to_resolution / 365)`. Polymarket's 4% yield applies only to 13 specific aggregate markets, not individual race contracts. |\n",
        simulator.kalshi_apy * 100.0,
    ));
    out.push_str("| **Projected future arb** | For each market: `projected_reentries × avg_profit_per_entry`. Projected re-entries use the sqrt discount model (see below). |\n");
    out.push_str("| **Projected future yield** | For each market: `projected_reentries × avg_capital_per_entry × 0.5 × kalshi_APY × (days_remaining / 2) / 365`. Factor of 0.5 because only the Kalshi leg earns yield. Half remaining days because future positions open over time. |\n");
    out.push_str("| **Projected total return** | `realized_arb + yield_on_positions + projected_future_arb + projected_future_yield` |\n\n");

    out.push_str("### Capital Requirements\n\n");
    out.push_str("| Metric | Calculation |\n");
    out.push_str("|--------|-------------|\n");
    out.push_str("| **Current capital deployed** | Sum of `entry_cost` across all open paper positions. This is the total capital locked in arb trades right now. |\n");
    out.push_str("| **Peak capital observed** | Highest value of current capital deployed seen during this session. Updated every time a new paper trade opens. |\n");
    out.push_str("| **Projected peak capital** | `peak_capital_observed + sum(projected_reentries × avg_capital_per_entry)` across all markets. Assumes worst case where all projected future positions are open simultaneously. |\n");
    out.push_str("| **Projected ROI** | `projected_total_return / capital × 100%`. Shown for both current and projected peak capital. |\n\n");

    out.push_str("### Trade Entry Criteria\n\n");
    out.push_str("A paper trade executes when **all** of the following are true:\n\n");
    out.push_str("1. A **new** cross-platform arb window opens (spread was previously zero or negative)\n");
    out.push_str("2. **NEV >= 3.0 cents per share** — Net Expected Value after walking both order books and applying platform fees\n");
    out.push_str("3. **Executable depth >= $200** — enough liquidity to fill at least $200 of entry cost at profitable prices\n\n");
    out.push_str("NEV (Net Expected Value) = `(exit_proceeds - entry_cost) / shares_filled`. It represents the per-share profit after fees, computed by matching entry asks against exit bids at each price level.\n\n");

    out.push_str("### Re-entry Projection Model\n\n");
    out.push_str("We project how many more times each market will produce a tradeable arb before resolution:\n\n");
    out.push_str("```\n");
    out.push_str("projected_reentries = observed_entries × sqrt(days_remaining / days_observed)\n");
    out.push_str("```\n\n");
    out.push_str("**Why sqrt?** A linear extrapolation (\"5 entries in 2 days = 2.5/day for 200 days = 500 entries\") overstates the opportunity because arb frequency declines as markets approach resolution — prices converge and liquidity drops. The square root function provides diminishing marginal returns: the first 100 days of remaining life contribute more projected entries than the next 100 days.\n\n");
    out.push_str("**Minimum data requirement:** Projections require at least 2 observed entries and 1 hour of observation. Markets below this threshold show 0 projected entries.\n\n");

    out.push_str("### Positions\n\n");
    out.push_str("All positions are **held to market resolution** — no early exits. This is because:\n\n");
    out.push_str("1. Both platforms pay yield on open positions, so holding earns additional return\n");
    out.push_str("2. The arb is locked in at entry — the profit is guaranteed regardless of price movement\n");
    out.push_str("3. Early exit would require paying the spread again, eating into profit\n\n");

    out.push_str("### Key Assumptions & Limitations\n\n");
    out.push_str("- **Understated profits**: In reality, executing a trade closes the spread, which can then reopen for another trade. Paper trading doesn't close spreads, so we see fewer re-entry opportunities than real execution would produce.\n");
    out.push_str("- **No slippage modeled**: Paper trades fill at current book prices. Real execution may experience slippage, partial fills, or failed legs.\n");
    out.push_str("- **Yield rates are variable**: Platform APY rates can change. Current rates are snapshotted at session start.\n");
    out.push_str("- **Capital projection is worst-case**: Projected peak assumes all future positions overlap. In practice, some markets resolve before others, freeing capital.\n");

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
