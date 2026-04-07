# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-07 13:46:37 UTC_

**Session**: 8.7h | **Markets tracked**: 3 | **Positions**: 3

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$65.81** |
| Yield on positions (to resolution) | **$184.59** |
| **Realized + yield** | **$250.40** |
| Projected future arb (sqrt-discounted) | $0.00 |
| Projected future yield | $0.00 |
| **Projected total return** | **$250.40** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $1771.20 |
| Peak capital observed | $1771.20 |
| Projected peak capital (incl. re-entries) | **$1771.20** |
| Projected ROI on current capital | 14.1% |
| Projected ROI on projected peak capital | 14.1% |

## Platform Yield Rates

| Platform | APY | Source |
|----------|-----|--------|
| Kalshi | 3.5% | Interest on cash + positions |
| Polymarket | 4.0% | Position yield on eligible markets |

_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_IL16 | 1 | $660.00 | $25.00 | $79.36 | 575 | PM→K |
| HOUSE_TN07 | 1 | $667.50 | $25.00 | $80.24 | 575 | PM→K |
| CN-PHYSICAL_PRESENCE | 1 | $443.70 | $15.81 | $24.99 | 269 | K→PM |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| HOUSE_IL16 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| CN-PHYSICAL_PRESENCE | 1 | 0.0h | 0.0/d | 0 | $15.81 | $0.00 |
| HOUSE_TN07 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 05:10:14 | CN-PHYSICAL_PRESENCE | 510 | $443.70 | $15.81 | 3.1¢ | K→PM |
| 05:08:35 | HOUSE_IL16 | 750 | $660.00 | $25.00 | 3.3¢ | PM→K |
| 05:07:05 | HOUSE_TN07 | 750 | $667.50 | $25.00 | 3.3¢ | PM→K |

---

### Methodology

- **Paper trades** execute when cross-platform spread exceeds 3¢ NEV with $200+ depth
- **Positions are held to market resolution** — no early exits
- **Yield**: Kalshi 3.5% APY on positions + cash; Polymarket 4.0% APY on eligible markets
- **Re-entry projection**: `observed_count × √(days_remaining / days_observed)` — conservative sqrt discount
- **Capital projection**: peak observed + (projected re-entries × avg capital per entry)
- **Understated profits**: real execution would close spreads, allowing them to reopen for additional trades
