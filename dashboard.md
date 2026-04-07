# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-07 04:41:35 UTC_

**Session**: 12m | **Markets tracked**: 1 | **Positions**: 1

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$25.00** |
| Yield on positions (to resolution) | **$77.59** |
| **Realized + yield** | **$102.59** |
| Projected future arb (sqrt-discounted) | $0.00 |
| Projected future yield | $0.00 |
| **Projected total return** | **$102.59** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $645.00 |
| Peak capital observed | $645.00 |
| Projected peak capital (incl. re-entries) | **$645.00** |
| Projected ROI on current capital | 15.9% |
| Projected ROI on projected peak capital | 15.9% |

## Platform Yield Rates

| Platform | APY | Source |
|----------|-----|--------|
| Kalshi | 3.5% | Interest on cash + positions |
| Polymarket | 4.0% | Position yield on eligible markets |

_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_GA-12 | 1 | $645.00 | $25.00 | $77.59 | 575 | PM→K |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| HOUSE_GA-12 | 1 | 0.0h | 0.0/d | 0 | $25.00 | $0.00 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 04:38:09 | HOUSE_GA-12 | 750 | $645.00 | $25.00 | 3.3¢ | PM→K |

---

### Methodology

- **Paper trades** execute when cross-platform spread exceeds 3¢ NEV with $200+ depth
- **Positions are held to market resolution** — no early exits
- **Yield**: Kalshi 3.5% APY on positions + cash; Polymarket 4.0% APY on eligible markets
- **Re-entry projection**: `observed_count × √(days_remaining / days_observed)` — conservative sqrt discount
- **Capital projection**: peak observed + (projected re-entries × avg capital per entry)
- **Understated profits**: real execution would close spreads, allowing them to reopen for additional trades
