# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-11 14:45:41 UTC_

**Session**: 3d 22h | **Markets with positions**: 14 | **Total paper trades**: 18

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$237.67** |
| Yield on positions (to resolution) | **$132.04** |
| **Realized + yield** | **$369.71** |
| Projected future arb (sqrt-discounted) | $1256.23 |
| Projected future yield | $166.24 |
| **Projected total return** | **$1792.17** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $6281.08 |
| Peak capital observed | $6281.08 |
| Projected peak capital (incl. re-entries) | **$39625.60** |
| Projected ROI on current capital | 28.5% |
| Projected ROI on projected peak capital | 4.5% |

## Platform Yield Rates

| Platform | APY | Applies To |
|----------|-----|------------|
| Kalshi | 3.5% | All positions + cash (min $250 balance) |
| Polymarket | 4.0% | **13 specific aggregate markets only** (not individual races) |

_Our individual race arbs earn yield only on the Kalshi leg. Polymarket's 4% yield is limited to broad markets like "Balance of Power: 2026 Midterms" and "Which party wins the House?", not individual district races._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_AZ07 | 1 | $227.50 | $8.50 | $9.85 | 207 | PM→K |
| HOUSE_CA07 | 1 | $455.00 | $15.00 | $19.75 | 208 | PM→K |
| HOUSE_IL-02 | 1 | $235.00 | $8.00 | $10.15 | 207 | PM→K |
| HOUSE_MA08 | 1 | $232.50 | $8.25 | $10.05 | 207 | PM→K |
| HOUSE_MD04 | 2 | $465.00 | $18.50 | $20.10 | 207 | PM→K |
| HOUSE_NJ10 | 1 | $232.50 | $8.00 | $10.05 | 207 | PM→K |
| HOUSE_GA-09 | 2 | $920.00 | $32.00 | $40.01 | 209 | PM→K |
| HOUSE_GA-12 | 1 | $645.00 | $25.00 | $28.20 | 209 | PM→K |
| HOUSE_GA07 | 2 | $437.50 | $16.75 | $18.94 | 207 | PM→K |
| HOUSE_IL16 | 1 | $660.00 | $25.00 | $28.84 | 209 | PM→K |
| HOUSE_PA09 | 1 | $230.00 | $11.00 | $10.00 | 207 | PM→K |
| HOUSE_TN07 | 2 | $890.00 | $33.75 | $38.80 | 209 | PM→K |
| HOUSE_TX36 | 1 | $220.00 | $8.50 | $9.48 | 206 | PM→K |
| CN-PHYSICAL_PRESENCE | 1 | $431.08 | $19.42 | $24.31 | 268 | K→PM |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| HOUSE_GA07 | 2 | 8.1h | 5.9/d | 49 | $8.38 | $414.40 |
| HOUSE_TN07 | 2 | 56.7h | 0.8/d | 19 | $16.88 | $317.44 |
| HOUSE_MD04 | 2 | 25.9h | 1.9/d | 28 | $9.25 | $256.30 |
| HOUSE_GA-09 | 2 | 71.5h | 0.7/d | 17 | $16.00 | $268.09 |
| HOUSE_NJ10 | 1 | 0.0h | 0.0/d | 0 | $8.00 | $0.00 |
| HOUSE_CA07 | 1 | 0.0h | 0.0/d | 0 | $15.00 | $0.00 |
| HOUSE_AZ07 | 1 | 0.0h | 0.0/d | 0 | $8.50 | $0.00 |
| HOUSE_PA09 | 1 | 0.0h | 0.0/d | 0 | $11.00 | $0.00 |
| HOUSE_TX36 | 1 | 0.0h | 0.0/d | 0 | $8.50 | $0.00 |
| CN-PHYSICAL_PRESENCE | 1 | 0.0h | 0.0/d | 0 | $19.42 | $0.00 |
| HOUSE_GA-12 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_MA08 | 1 | 0.0h | 0.0/d | 0 | $8.25 | $0.00 |
| HOUSE_IL-02 | 1 | 0.0h | 0.0/d | 0 | $8.00 | $0.00 |
| HOUSE_IL16 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 02:56:08 | HOUSE_MD04 | 250 | $232.50 | $8.25 | 3.3¢ | PM→K |
| 02:49:37 | HOUSE_TX36 | 250 | $220.00 | $8.50 | 3.4¢ | PM→K |
| 15:58:36 | HOUSE_GA-09 | 250 | $230.00 | $9.50 | 3.8¢ | PM→K |
| 09:56:15 | HOUSE_PA09 | 250 | $230.00 | $11.00 | 4.4¢ | PM→K |
| 09:12:46 | HOUSE_GA07 | 250 | $217.50 | $8.00 | 3.2¢ | PM→K |
| 01:07:59 | HOUSE_AZ07 | 250 | $227.50 | $8.50 | 3.4¢ | PM→K |
| 01:05:32 | HOUSE_TN07 | 250 | $222.50 | $8.75 | 3.5¢ | PM→K |
| 01:05:23 | HOUSE_GA07 | 250 | $220.00 | $8.75 | 3.5¢ | PM→K |
| 01:03:20 | HOUSE_NJ10 | 250 | $232.50 | $8.00 | 3.2¢ | PM→K |
| 01:03:08 | HOUSE_IL-02 | 250 | $235.00 | $8.00 | 3.2¢ | PM→K |
| 01:03:06 | HOUSE_MA08 | 250 | $232.50 | $8.25 | 3.3¢ | PM→K |
| 01:03:06 | HOUSE_MD04 | 250 | $232.50 | $10.25 | 4.1¢ | PM→K |
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
