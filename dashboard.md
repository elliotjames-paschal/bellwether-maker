# Bellwether Maker — Paper Trading Dashboard

_Last updated: 2026-04-06 14:32:18 UTC_

**Session**: 2d 7h | **Markets tracked**: 12 | **Positions**: 47

## P&L Summary

| Metric | Value |
|--------|-------|
| Realized arb profit | **$847.32** |
| Yield on positions (to resolution) | **$312.55** |
| **Realized + yield** | **$1,159.87** |
| Projected future arb (sqrt-discounted) | $2,415.80 |
| Projected future yield | $890.22 |
| **Projected total return** | **$4,465.89** |

## Capital Requirements

| Metric | Value |
|--------|-------|
| Current capital deployed | $8,240.50 |
| Peak capital observed | $9,715.00 |
| Projected peak capital (incl. re-entries) | **$18,430.00** |
| Projected ROI on current capital | 54.2% |
| Projected ROI on projected peak capital | 24.2% |

## Platform Yield Rates

| Platform | APY | Source |
|----------|-----|--------|
| Kalshi | 3.5% | Interest on cash + positions |
| Polymarket | 4.0% | Position yield on eligible markets |

_Both legs of each arb earn yield independently. Entry cost earns yield on the entry platform; exit proceeds earn yield on the exit platform._

## Open Positions

| Market | Entries | Capital | Arb Profit | Yield (est.) | Days Left | Direction |
|--------|---------|---------|------------|-------------|-----------|----------|
| HOUSE_GA-12 | 8 | $1,280.50 | $132.00 | $47.20 | 580 | PM→K |
| HOUSE_TX36 | 6 | $990.00 | $100.00 | $36.50 | 580 | PM→K |
| SENATE_AK | 7 | $870.25 | $78.40 | $32.10 | 580 | K→PM |
| HOUSE_GA-02 | 5 | $825.00 | $68.75 | $30.40 | 580 | PM→K |
| HOUSE_TN07 | 4 | $668.00 | $50.00 | $24.60 | 580 | PM→K |
| SEN_ME | 4 | $742.00 | $73.96 | $27.35 | 580 | K→PM |
| HOUSE_SC07 | 3 | $535.50 | $43.50 | $19.75 | 580 | PM→K |
| HOUSE_OH_08 | 3 | $475.00 | $37.50 | $17.50 | 580 | PM→K |
| HOUSE_FL21 | 2 | $652.00 | $40.00 | $24.05 | 580 | PM→K |
| GOV_NV | 2 | $615.00 | $35.00 | $22.70 | 580 | K→PM |
| HOUSE_WI02 | 2 | $368.00 | $19.50 | $13.57 | 580 | PM→K |
| GOV_GA | 1 | $219.25 | $168.71 | $16.83 | 580 | K→PM |

## Re-entry Analysis

_Re-entry projections use sqrt discount: `projected = observed × √(days_remaining / days_observed)`. This assumes arb frequency diminishes as markets approach resolution._

| Market | Observed | Obs. Hours | Rate/Day | Projected | Avg Profit | Projected Profit |
|--------|----------|------------|----------|-----------|------------|------------------|
| HOUSE_GA-12 | 8 | 55.0h | 3.5/d | 128 | $16.50 | $2,112.00 |
| SENATE_AK | 7 | 55.0h | 3.1/d | 112 | $11.20 | $1,254.40 |
| HOUSE_TX36 | 6 | 55.0h | 2.6/d | 96 | $16.67 | $1,600.32 |
| HOUSE_GA-02 | 5 | 55.0h | 2.2/d | 80 | $13.75 | $1,100.00 |
| SEN_ME | 4 | 48.0h | 2.0/d | 68 | $18.49 | $1,257.32 |
| HOUSE_TN07 | 4 | 55.0h | 1.7/d | 64 | $12.50 | $800.00 |
| HOUSE_SC07 | 3 | 55.0h | 1.3/d | 48 | $14.50 | $696.00 |
| HOUSE_OH_08 | 3 | 40.0h | 1.8/d | 51 | $12.50 | $637.50 |
| HOUSE_FL21 | 2 | 30.0h | 1.6/d | 38 | $20.00 | $760.00 |
| GOV_NV | 2 | 30.0h | 1.6/d | 38 | $17.50 | $665.00 |
| HOUSE_WI02 | 2 | 24.0h | 2.0/d | 40 | $9.75 | $390.00 |
| GOV_GA | 1 | 12.0h | 2.0/d | 22 | $168.71 | $3,711.62 |

## Recent Paper Trades (last 20)

| Time (UTC) | Market | Shares | Capital | Profit | NEV | Direction |
|------------|--------|--------|---------|--------|-----|----------|
| 14:31:05 | HOUSE_GA-12 | 650 | $162.50 | $16.25 | 3.7¢ | PM→K |
| 14:12:48 | SENATE_AK | 340 | $124.10 | $11.56 | 3.4¢ | K→PM |
| 13:55:22 | HOUSE_TX36 | 660 | $165.00 | $16.50 | 3.3¢ | PM→K |
| 13:41:17 | HOUSE_GA-02 | 660 | $165.00 | $13.75 | 3.7¢ | PM→K |
| 13:28:03 | SEN_ME | 371 | $185.50 | $18.49 | 3.6¢ | K→PM |
| 13:15:44 | HOUSE_TN07 | 668 | $167.00 | $12.50 | 3.3¢ | PM→K |
| 12:52:19 | HOUSE_GA-12 | 645 | $161.25 | $16.12 | 4.3¢ | PM→K |
| 12:38:56 | HOUSE_SC07 | 675 | $178.50 | $14.50 | 2.8¢ | PM→K |
| 12:15:33 | HOUSE_OH_08 | 95 | $158.33 | $12.50 | 3.3¢ | PM→K |
| 11:58:07 | HOUSE_TX36 | 660 | $165.00 | $16.50 | 3.3¢ | PM→K |
| 11:42:41 | HOUSE_GA-12 | 650 | $162.50 | $16.25 | 3.7¢ | PM→K |
| 11:25:18 | SENATE_AK | 335 | $122.28 | $11.22 | 3.4¢ | K→PM |
| 11:08:52 | GOV_NV | 123 | $307.50 | $17.50 | 2.5¢ | K→PM |
| 10:52:29 | HOUSE_GA-02 | 660 | $165.00 | $13.75 | 3.7¢ | PM→K |
| 10:35:03 | HOUSE_FL21 | 652 | $326.00 | $20.00 | 2.7¢ | PM→K |
| 10:18:40 | HOUSE_WI02 | 368 | $184.00 | $9.75 | 2.4¢ | PM→K |
| 10:02:14 | HOUSE_GA-12 | 645 | $161.25 | $16.12 | 4.3¢ | PM→K |
| 09:45:51 | HOUSE_TN07 | 668 | $167.00 | $12.50 | 3.3¢ | PM→K |
| 09:28:28 | SEN_ME | 371 | $185.50 | $18.49 | 3.6¢ | K→PM |
| 09:12:05 | HOUSE_TX36 | 660 | $165.00 | $16.50 | 3.3¢ | PM→K |

---

### Methodology

- **Paper trades** execute when cross-platform spread exceeds 3¢ NEV with $200+ depth
- **Positions are held to market resolution** — no early exits
- **Yield**: Kalshi 3.5% APY on positions + cash; Polymarket 4.0% APY on eligible markets
- **Re-entry projection**: `observed_count × √(days_remaining / days_observed)` — conservative sqrt discount
- **Capital projection**: peak observed + (projected re-entries × avg capital per entry)
- **Understated profits**: real execution would close spreads, allowing them to reopen for additional trades
