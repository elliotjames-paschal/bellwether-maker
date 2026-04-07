# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-07 15:57:26 UTC_

**Session**: 5m | **Markets with positions**: 6 | **Total paper trades**: 6

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$134.65** |
| Yield on positions (to resolution) | **$444.94** |
| **Realized + yield** | **$579.59** |
| Projected future arb (sqrt-discounted) | $0.00 |
| Projected future yield | $0.00 |
| **Projected total return** | **$579.59** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $3709.64 |
| Peak capital observed | $3709.64 |
| Projected peak capital (incl. re-entries) | **$3709.64** |
| Projected ROI on current capital | 15.6% |
| Projected ROI on projected peak capital | 15.6% |

## Platform Yield Rates

| Platform | APY | Source |
|----------|-----|--------|
| Kalshi | 3.5% | Interest on cash + positions |
| Polymarket | 4.0% | Position yield on eligible markets |

_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_MA09 | 1 | $690.00 | $22.50 | $82.62 | 574 | PM→K |
| HOUSE_GA-09 | 1 | $690.00 | $22.50 | $82.62 | 574 | PM→K |
| HOUSE_GA-12 | 1 | $645.00 | $25.00 | $77.45 | 574 | PM→K |
| HOUSE_TN07 | 1 | $667.50 | $25.00 | $80.10 | 574 | PM→K |
| HOUSE_TX36 | 1 | $660.00 | $25.00 | $79.22 | 574 | PM→K |
| HOUSE_WI07 | 1 | $357.13 | $14.65 | $42.93 | 574 | PM→K |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| HOUSE_WI07 | 1 | 0.0h | 0.0/d | 0 | $14.65 | $0.00 |
| HOUSE_TX36 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_MA09 | 1 | 0.0h | 0.0/d | 0 | $22.50 | $0.00 |
| HOUSE_TN07 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_GA-12 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_GA-09 | 1 | 0.0h | 0.0/d | 0 | $22.50 | $0.00 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 15:56:31 | HOUSE_TN07 | 750 | $667.50 | $25.00 | 3.3¢ | PM→K |
| 15:55:53 | HOUSE_GA-09 | 750 | $690.00 | $22.50 | 3.0¢ | PM→K |
| 15:55:53 | HOUSE_MA09 | 750 | $690.00 | $22.50 | 3.0¢ | PM→K |
| 15:55:17 | HOUSE_TX36 | 750 | $660.00 | $25.00 | 3.3¢ | PM→K |
| 15:53:44 | HOUSE_WI07 | 410 | $357.13 | $14.65 | 3.6¢ | PM→K |
| 15:53:13 | HOUSE_GA-12 | 750 | $645.00 | $25.00 | 3.3¢ | PM→K |

---

### Methodology

- **Paper trades** execute when cross-platform spread exceeds 3¢ NEV with $200+ depth
- **Positions are held to market resolution** — no early exits
- **Yield**: Kalshi 3.5% APY on positions + cash; Polymarket 4.0% APY on eligible markets
- **Re-entry projection**: `observed_count × √(days_remaining / days_observed)` — conservative sqrt discount
- **Capital projection**: peak observed + (projected re-entries × avg capital per entry)
- **Understated profits**: real execution would close spreads, allowing them to reopen for additional trades
