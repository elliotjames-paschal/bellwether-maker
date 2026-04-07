# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-07 05:05:54 UTC_

**Session**: 36m | **Markets tracked**: 4 | **Positions**: 4

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$323.09** |
| Yield on positions (to resolution) | **$261.44** |
| **Realized + yield** | **$584.54** |
| Projected future arb (sqrt-discounted) | $0.00 |
| Projected future yield | $0.00 |
| **Projected total return** | **$584.54** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $2179.36 |
| Peak capital observed | $2179.36 |
| Projected peak capital (incl. re-entries) | **$2179.36** |
| Projected ROI on current capital | 26.8% |
| Projected ROI on projected peak capital | 26.8% |

## Platform Yield Rates

| Platform | APY | Source |
|----------|-----|--------|
| Kalshi | 3.5% | Interest on cash + positions |
| Polymarket | 4.0% | Position yield on eligible markets |

_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_MD04 | 1 | $697.50 | $25.00 | $83.79 | 575 | PM→K |
| HOUSE_GA-12 | 1 | $645.00 | $25.00 | $77.59 | 575 | PM→K |
| HOUSE_NJ11 | 1 | $249.61 | $251.09 | $29.47 | 374 | K→PM |
| HOUSE_WI07 | 1 | $587.25 | $22.00 | $70.60 | 575 | PM→K |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| HOUSE_GA-12 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_MD04 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |
| HOUSE_NJ11 | 1 | 0.0h | 0.0/d | 0 | $251.09 | $0.00 |
| HOUSE_WI07 | 1 | 0.0h | 0.0/d | 0 | $22.00 | $0.00 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 04:53:45 | HOUSE_NJ11 | 6259 | $249.61 | $251.09 | 4.0¢ | K→PM |
| 04:52:37 | HOUSE_WI07 | 675 | $587.25 | $22.00 | 3.3¢ | PM→K |
| 04:52:34 | HOUSE_MD04 | 750 | $697.50 | $25.00 | 3.3¢ | PM→K |
| 04:38:09 | HOUSE_GA-12 | 750 | $645.00 | $25.00 | 3.3¢ | PM→K |

---

### Methodology

- **Paper trades** execute when cross-platform spread exceeds 3¢ NEV with $200+ depth
- **Positions are held to market resolution** — no early exits
- **Yield**: Kalshi 3.5% APY on positions + cash; Polymarket 4.0% APY on eligible markets
- **Re-entry projection**: `observed_count × √(days_remaining / days_observed)` — conservative sqrt discount
- **Capital projection**: peak observed + (projected re-entries × avg capital per entry)
- **Understated profits**: real execution would close spreads, allowing them to reopen for additional trades
