# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-09 13:15:16 UTC_

**Session**: 1d 20h | **Markets with positions**: 6 | **Total paper trades**: 6

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$131.92** |
| Yield on positions (to resolution) | **$75.77** |
| **Realized + yield** | **$207.69** |
| Projected future arb (sqrt-discounted) | $0.00 |
| Projected future yield | $0.00 |
| **Projected total return** | **$207.69** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $3548.58 |
| Peak capital observed | $3548.58 |
| Projected peak capital (incl. re-entries) | **$3548.58** |
| Projected ROI on current capital | 5.9% |
| Projected ROI on projected peak capital | 5.9% |

## Platform Yield Rates

| Platform | APY | Applies To |
|----------|-----|------------|
| Kalshi | 3.5% | All positions + cash (min $250 balance) |
| Polymarket | 4.0% | **13 specific aggregate markets only** (not individual races) |

_Our individual race arbs earn yield only on the Kalshi leg. Polymarket's 4% yield is limited to broad markets like "Balance of Power: 2026 Midterms" and "Which party wins the House?", not individual district races._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_CA07 | 1 | $455.00 | $15.00 | $19.75 | 208 | PM→K |
| HOUSE_GA-09 | 1 | $690.00 | $22.50 | $30.08 | 209 | PM→K |
| HOUSE_GA-12 | 1 | $645.00 | $25.00 | $28.20 | 209 | PM→K |
| HOUSE_IL16 | 1 | $660.00 | $25.00 | $28.84 | 209 | PM→K |
| HOUSE_TN07 | 1 | $667.50 | $25.00 | $29.17 | 209 | PM→K |
| CN-PHYSICAL_PRESENCE | 1 | $431.08 | $19.42 | $24.31 | 268 | K→PM |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| CN-PHYSICAL_PRESENCE | 1 | 0.0h | 0.0/d | 0 | $19.42 | $0.00 |
| HOUSE_GA-12 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_CA07 | 1 | 0.0h | 0.0/d | 0 | $15.00 | $0.00 |
| HOUSE_TN07 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_IL16 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_GA-09 | 1 | 0.0h | 0.0/d | 0 | $22.50 | $0.00 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 12:18:00 | HOUSE_CA07 | 500 | $455.00 | $15.00 | 3.0¢ | PM→K |
| 16:38:35 | CN-PHYSICAL_PRESENCE | 500 | $431.08 | $19.42 | 3.9¢ | K→PM |
| 16:30:29 | HOUSE_GA-09 | 750 | $690.00 | $22.50 | 3.0¢ | PM→K |
| 16:23:21 | HOUSE_IL16 | 750 | $660.00 | $25.00 | 3.3¢ | PM→K |
| 16:23:10 | HOUSE_GA-12 | 750 | $645.00 | $25.00 | 3.3¢ | PM→K |
| 16:22:42 | HOUSE_TN07 | 750 | $667.50 | $25.00 | 3.3¢ | PM→K |

---

## How Each Metric Is Calculated

### P&L Summary

| Metric | Calculation |
|--------|-------------|
| **Realized arb profit** | Sum of `net_profit` across all paper trades. Each trade's profit = `exit_proceeds - entry_cost`, computed by walking both order books and matching shares at each price level until the spread is consumed. |
| **Yield on positions** | For each position, only the Kalshi leg earns yield: `kalshi_leg_value × 3.5% × (days_to_resolution / 365)`. Polymarket's 4% yield applies only to 13 specific aggregate markets, not individual race contracts. |
| **Projected future arb** | For each market: `projected_reentries × avg_profit_per_entry`. Projected re-entries use the sqrt discount model (see below). |
| **Projected future yield** | For each market: `projected_reentries × avg_capital_per_entry × 0.5 × kalshi_APY × (days_remaining / 2) / 365`. Factor of 0.5 because only the Kalshi leg earns yield. Half remaining days because future positions open over time. |
| **Projected total return** | `realized_arb + yield_on_positions + projected_future_arb + projected_future_yield` |

### Capital Requirements

| Metric | Calculation |
|--------|-------------|
| **Current capital deployed** | Sum of `entry_cost` across all open paper positions. This is the total capital locked in arb trades right now. |
| **Peak capital observed** | Highest value of current capital deployed seen during this session. Updated every time a new paper trade opens. |
| **Projected peak capital** | `peak_capital_observed + sum(projected_reentries × avg_capital_per_entry)` across all markets. Assumes worst case where all projected future positions are open simultaneously. |
| **Projected ROI** | `projected_total_return / capital × 100%`. Shown for both current and projected peak capital. |

### Trade Entry Criteria

A paper trade executes when **all** of the following are true:

1. A **new** cross-platform arb window opens (spread was previously zero or negative)
2. **NEV >= 3.0 cents per share** — Net Expected Value after walking both order books and applying platform fees
3. **Executable depth >= $200** — enough liquidity to fill at least $200 of entry cost at profitable prices

NEV (Net Expected Value) = `(exit_proceeds - entry_cost) / shares_filled`. It represents the per-share profit after fees, computed by matching entry asks against exit bids at each price level.

### Re-entry Projection Model

We project how many more times each market will produce a tradeable arb before resolution:

```
projected_reentries = observed_entries × sqrt(days_remaining / days_observed)
```

**Why sqrt?** A linear extrapolation ("5 entries in 2 days = 2.5/day for 200 days = 500 entries") overstates the opportunity because arb frequency declines as markets approach resolution — prices converge and liquidity drops. The square root function provides diminishing marginal returns: the first 100 days of remaining life contribute more projected entries than the next 100 days.

**Minimum data requirement:** Projections require at least 2 observed entries and 1 hour of observation. Markets below this threshold show 0 projected entries.

### Positions

All positions are **held to market resolution** — no early exits. This is because:

1. Both platforms pay yield on open positions, so holding earns additional return
2. The arb is locked in at entry — the profit is guaranteed regardless of price movement
3. Early exit would require paying the spread again, eating into profit

### Key Assumptions & Limitations

- **Understated profits**: In reality, executing a trade closes the spread, which can then reopen for another trade. Paper trading doesn't close spreads, so we see fewer re-entry opportunities than real execution would produce.
- **No slippage modeled**: Paper trades fill at current book prices. Real execution may experience slippage, partial fills, or failed legs.
- **Yield rates are variable**: Platform APY rates can change. Current rates are snapshotted at session start.
- **Capital projection is worst-case**: Projected peak assumes all future positions overlap. In practice, some markets resolve before others, freeing capital.
