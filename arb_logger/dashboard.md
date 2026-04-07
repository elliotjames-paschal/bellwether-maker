# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-07 04:30:14 UTC_

**Session**: 1m | **Markets tracked**: 0 | **Positions**: 0

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$0.00** |
| Yield on positions (to resolution) | **$0.00** |
| **Realized + yield** | **$0.00** |
| Projected future arb (sqrt-discounted) | $0.00 |
| Projected future yield | $0.00 |
| **Projected total return** | **$0.00** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $0.00 |
| Peak capital observed | $0.00 |
| Projected peak capital (incl. re-entries) | **$0.00** |

## Platform Yield Rates

| Platform | APY | Source |
|----------|-----|--------|
| Kalshi | 3.5% | Interest on cash + positions |
| Polymarket | 4.0% | Position yield on eligible markets |

_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._

## Open Positions

_No paper positions yet. Waiting for arb opportunities above threshold._

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

_No re-entry data yet._

## Recent Paper Trades (last 20)

_No trades yet._

---

### Methodology

- **Paper trades** execute when cross-platform spread exceeds 3¢ NEV with $200+ depth
- **Positions are held to market resolution** — no early exits
- **Yield**: Kalshi 3.5% APY on positions + cash; Polymarket 4.0% APY on eligible markets
- **Re-entry projection**: `observed_count × √(days_remaining / days_observed)` — conservative sqrt discount
- **Capital projection**: peak observed + (projected re-entries × avg capital per entry)
- **Understated profits**: real execution would close spreads, allowing them to reopen for additional trades
