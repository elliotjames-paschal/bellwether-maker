# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-07 16:13:52 UTC_

**Session**: 2m | **Markets with positions**: 2 | **Total paper trades**: 2

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$79.62** |
| Yield on positions (to resolution) | **$95.52** |
| **Realized + yield** | **$175.14** |
| Projected future arb (sqrt-discounted) | $0.00 |
| Projected future yield | $0.00 |
| **Projected total return** | **$175.14** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $2186.98 |
| Peak capital observed | $2186.98 |
| Projected peak capital (incl. re-entries) | **$2186.98** |
| Projected ROI on current capital | 8.0% |
| Projected ROI on projected peak capital | 8.0% |

## Platform Yield Rates

| Platform | APY | Source |
|----------|-----|--------|
| Kalshi | 3.5% | Interest on cash + positions |
| Polymarket | 4.0% | Position yield on eligible markets |

_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_GA-08 | 1 | $1526.98 | $54.62 | $66.67 | 209 | PM→K |
| HOUSE_TX36 | 1 | $660.00 | $25.00 | $28.84 | 209 | PM→K |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| HOUSE_GA-08 | 1 | 0.0h | 0.0/d | 0 | $54.62 | $0.00 |
| HOUSE_TX36 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 16:13:09 | HOUSE_GA-08 | 1678 | $1526.98 | $54.62 | 3.3¢ | PM→K |
| 16:11:52 | HOUSE_TX36 | 750 | $660.00 | $25.00 | 3.3¢ | PM→K |

---

## How Each Metric Is Calculated

### P&L Summary

| Metric | Calculation |
|--------|-------------|
| **Realized arb profit** | Sum of `net_profit` across all paper trades. Each trade's profit = `exit_proceeds - entry_cost`, computed by walking both order books and matching shares at each price level until the spread is consumed. |
| **Yield on positions** | For each position: `(entry_cost × entry_platform_APY + exit_proceeds × exit_platform_APY) × (days_to_resolution / 365)`. Both legs earn yield independently — Kalshi pays 3.5% APY on all positions, Polymarket pays 4.0% APY on eligible markets. |
| **Projected future arb** | For each market: `projected_reentries × avg_profit_per_entry`. Projected re-entries use the sqrt discount model (see below). |
| **Projected future yield** | For each market: `projected_reentries × avg_capital_per_entry × avg_APY × (days_remaining / 2) / 365`. Uses half the remaining days because future positions are opened over time, not all at once. |
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
