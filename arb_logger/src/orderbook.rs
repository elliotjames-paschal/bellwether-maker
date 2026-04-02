use crate::types::{FeeRates, FillEvent, LegDetail, PlatformBook, RoundTrip, RoundTripDetail};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

/// Snapshot a BTreeMap book side into Vec<(f64, f64)>.
fn snapshot_book(book: &BTreeMap<OrderedFloat<f64>, f64>) -> Vec<(f64, f64)> {
    book.iter().map(|(p, s)| (p.into_inner(), *s)).collect()
}

/// Simulate a round-trip arb trade. Returns both the simple RoundTrip (for
/// backward compat / scorer) and the full RoundTripDetail with book snapshots,
/// leg details, and standardization costs.
pub fn simulate_round_trip_btree(
    kalshi_book: &PlatformBook,
    polymarket_book: &PlatformBook,
    fees: &FeeRates,
) -> Option<(RoundTrip, RoundTripDetail)> {
    // Snapshot all four sides of the market at this instant
    let kalshi_yes_asks = snapshot_book(&kalshi_book.asks);
    let kalshi_no_asks: Vec<(f64, f64)> = kalshi_book
        .bids
        .iter()
        .map(|(p, s)| (1.0 - p.into_inner(), *s))
        .collect();
    let polymarket_yes_asks = snapshot_book(&polymarket_book.asks);
    let polymarket_no_asks: Vec<(f64, f64)> = polymarket_book
        .bids
        .iter()
        .map(|(p, s)| (1.0 - p.into_inner(), *s))
        .collect();

    // Direction A: buy YES on Polymarket asks, sell YES on Kalshi bids
    let dir_a = try_direction_btree(
        &polymarket_book.asks,
        &kalshi_book.bids,
        fees.polymarket_fee,
        fees.kalshi_fee,
        "polymarket",
        "kalshi",
        "YES",
        "YES",
    );

    // Direction B: buy YES on Kalshi asks, sell YES on Polymarket bids
    let dir_b = try_direction_btree(
        &kalshi_book.asks,
        &polymarket_book.bids,
        fees.kalshi_fee,
        fees.polymarket_fee,
        "kalshi",
        "polymarket",
        "YES",
        "YES",
    );

    // Complementary sanity check: for binary markets, YES and NO arbs are
    // equivalent by construction. Kalshi NO bids at price X are stored as YES
    // asks at (1-X) in ws.rs, so the YES walk already covers the NO side.
    // Validate that the conversion hasn't produced nonsensical prices.
    #[cfg(debug_assertions)]
    {
        for (price, _) in kalshi_book.asks.iter() {
            let p = price.into_inner();
            debug_assert!(
                p > 0.0 && p < 1.0,
                "Kalshi YES ask price {:.4} out of range — NO→YES conversion bug?",
                p
            );
        }
        for (price, _) in kalshi_book.bids.iter() {
            let p = price.into_inner();
            debug_assert!(
                p > 0.0 && p < 1.0,
                "Kalshi YES bid price {:.4} out of range",
                p
            );
        }
    }

    let winner = match (&dir_a, &dir_b) {
        (Some(a), Some(b)) => {
            let pa = a.0.total_exit_proceeds - a.0.total_entry_cost;
            let pb = b.0.total_exit_proceeds - b.0.total_entry_cost;
            if pa >= pb { dir_a } else { dir_b }
        }
        (Some(_), None) => dir_a,
        (None, Some(_)) => dir_b,
        (None, None) => return None,
    };

    let (rt, entry_leg, exit_leg, direction_label) = winner?;

    let gross_spread = rt.total_exit_proceeds - rt.total_entry_cost;
    // Fees are already applied per-share inside try_direction_btree:
    //   entry: ask_price * (1 + entry_fee)
    //   exit:  bid_price * (1 - exit_fee)
    // So total_entry_cost and total_exit_proceeds are post-fee figures.
    let fee_amount = 0.0;
    let net_profit = gross_spread - fee_amount;
    let nev_per_share = if rt.shares_filled > 0.0 {
        net_profit / rt.shares_filled
    } else {
        0.0
    };

    // Compute standardization costs:
    // cost_to_profitably_standardize = total_entry_cost (capital needed for profitable shares)
    // cost_to_fully_standardize = cost to walk entire remaining book until spread is zero
    let cost_to_profitably_standardize = rt.total_entry_cost;

    // Walk the full books to compute cost_to_fully_standardize
    let (full_cost, residual) = compute_full_standardization_cost(
        if rt.entry_platform == "polymarket" {
            &polymarket_book.asks
        } else {
            &kalshi_book.asks
        },
        if rt.exit_platform == "kalshi" {
            &kalshi_book.bids
        } else {
            &polymarket_book.bids
        },
        if rt.entry_platform == "polymarket" {
            fees.polymarket_fee
        } else {
            fees.kalshi_fee
        },
        if rt.exit_platform == "kalshi" {
            fees.kalshi_fee
        } else {
            fees.polymarket_fee
        },
    );

    let detail = RoundTripDetail {
        kalshi_yes_asks,
        kalshi_no_asks,
        polymarket_yes_asks,
        polymarket_no_asks,
        direction: direction_label,
        entry_leg,
        exit_leg,
        gross_spread,
        fees: fee_amount,
        net_profit,
        nev_per_share,
        cost_to_profitably_standardize,
        cost_to_fully_standardize: full_cost,
        residual_spread_after_trade: residual,
        fully_standardized: residual.abs() < 0.001,
    };

    Some((rt, detail))
}

/// Try a specific direction. Returns (RoundTrip, entry_leg, exit_leg, direction_label).
fn try_direction_btree(
    entry_asks: &BTreeMap<OrderedFloat<f64>, f64>,
    exit_bids: &BTreeMap<OrderedFloat<f64>, f64>,
    entry_fee: f64,
    exit_fee: f64,
    entry_platform: &str,
    exit_platform: &str,
    entry_side: &str,
    exit_side: &str,
) -> Option<(RoundTrip, LegDetail, LegDetail, String)> {
    if entry_asks.is_empty() || exit_bids.is_empty() {
        return None;
    }

    let ask_levels: Vec<(f64, f64)> = entry_asks
        .iter()
        .map(|(p, s)| (p.into_inner(), *s))
        .collect();
    let bid_levels: Vec<(f64, f64)> = exit_bids
        .iter()
        .rev()
        .map(|(p, s)| (p.into_inner(), *s))
        .collect();

    let mut entry_fills: Vec<FillEvent> = Vec::new();
    let mut exit_fills: Vec<FillEvent> = Vec::new();
    let mut total_entry_cost = 0.0;
    let mut total_exit_proceeds = 0.0;
    let mut shares_filled = 0.0;
    let mut shares_exited = 0.0;
    let mut entry_levels_consumed: u32 = 0;
    let mut exit_levels_consumed: u32 = 0;
    let mut entry_book_walked: Vec<(f64, f64)> = Vec::new();
    let mut exit_book_walked: Vec<(f64, f64)> = Vec::new();

    let mut ask_idx = 0;
    let mut bid_idx = 0;
    let mut ask_remaining = ask_levels[0].1;
    let mut bid_remaining = bid_levels[0].1;

    loop {
        if ask_idx >= ask_levels.len() || bid_idx >= bid_levels.len() {
            break;
        }

        let (ask_price, _) = ask_levels[ask_idx];
        let (bid_price, _) = bid_levels[bid_idx];

        let entry_cost_per_share = ask_price * (1.0 + entry_fee);
        let exit_proceeds_per_share = bid_price * (1.0 - exit_fee);

        if exit_proceeds_per_share - entry_cost_per_share <= 0.0 {
            break;
        }

        let fill_size = ask_remaining.min(bid_remaining);
        if fill_size <= 0.0 {
            break;
        }

        total_entry_cost += fill_size * entry_cost_per_share;
        total_exit_proceeds += fill_size * exit_proceeds_per_share;
        shares_filled += fill_size;
        shares_exited += fill_size;

        entry_fills.push(FillEvent {
            price: ask_price,
            size: fill_size,
            platform: entry_platform.to_string(),
        });
        exit_fills.push(FillEvent {
            price: bid_price,
            size: fill_size,
            platform: exit_platform.to_string(),
        });

        // Track which book levels we consumed
        if entry_book_walked.last().map(|(p, _)| *p) != Some(ask_price) {
            entry_book_walked.push((ask_price, fill_size));
            entry_levels_consumed += 1;
        } else if let Some(last) = entry_book_walked.last_mut() {
            last.1 += fill_size;
        }
        if exit_book_walked.last().map(|(p, _)| *p) != Some(bid_price) {
            exit_book_walked.push((bid_price, fill_size));
            exit_levels_consumed += 1;
        } else if let Some(last) = exit_book_walked.last_mut() {
            last.1 += fill_size;
        }

        ask_remaining -= fill_size;
        bid_remaining -= fill_size;

        if ask_remaining <= f64::EPSILON {
            ask_idx += 1;
            if ask_idx < ask_levels.len() {
                ask_remaining = ask_levels[ask_idx].1;
            }
        }
        if bid_remaining <= f64::EPSILON {
            bid_idx += 1;
            if bid_idx < bid_levels.len() {
                bid_remaining = bid_levels[bid_idx].1;
            }
        }
    }

    let net_profit = total_exit_proceeds - total_entry_cost;
    if net_profit <= 0.0 || shares_filled <= 0.0 {
        return None;
    }

    let avg_entry = total_entry_cost / shares_filled;
    let avg_exit = total_exit_proceeds / shares_exited;

    let direction_label = format!(
        "BUY_{}_{}_{}_{}",
        entry_side,
        entry_platform.to_uppercase(),
        exit_side,
        exit_platform.to_uppercase()
    );

    let entry_leg = LegDetail {
        platform: entry_platform.to_string(),
        side: entry_side.to_string(),
        action: "BUY".to_string(),
        shares: shares_filled,
        avg_price: avg_entry,
        total_cost: total_entry_cost,
        levels_consumed: entry_levels_consumed,
        book_snapshot: entry_book_walked,
    };

    let exit_leg = LegDetail {
        platform: exit_platform.to_string(),
        side: exit_side.to_string(),
        action: "BUY".to_string(),
        shares: shares_exited,
        avg_price: avg_exit,
        total_cost: total_exit_proceeds,
        levels_consumed: exit_levels_consumed,
        book_snapshot: exit_book_walked,
    };

    let rt = RoundTrip {
        direction: format!("BUY on {} → SELL on {}", entry_platform, exit_platform),
        entry_platform: entry_platform.to_string(),
        exit_platform: exit_platform.to_string(),
        entry_fills,
        exit_fills,
        total_entry_cost,
        total_exit_proceeds,
        shares_filled,
        shares_exited,
    };

    Some((rt, entry_leg, exit_leg, direction_label))
}

/// Walk the full books to compute cost to fully standardize (drive spread to zero)
/// and the residual spread after the profitable portion is consumed.
fn compute_full_standardization_cost(
    entry_asks: &BTreeMap<OrderedFloat<f64>, f64>,
    exit_bids: &BTreeMap<OrderedFloat<f64>, f64>,
    entry_fee: f64,
    exit_fee: f64,
) -> (f64, f64) {
    let ask_levels: Vec<(f64, f64)> = entry_asks
        .iter()
        .map(|(p, s)| (p.into_inner(), *s))
        .collect();
    let bid_levels: Vec<(f64, f64)> = exit_bids
        .iter()
        .rev()
        .map(|(p, s)| (p.into_inner(), *s))
        .collect();

    if ask_levels.is_empty() || bid_levels.is_empty() {
        return (0.0, 0.0);
    }

    let mut total_cost = 0.0;
    let mut ask_idx = 0;
    let mut bid_idx = 0;
    let mut ask_remaining = ask_levels[0].1;
    let mut bid_remaining = bid_levels[0].1;

    // Walk until ask price >= bid price (no more spread)
    loop {
        if ask_idx >= ask_levels.len() || bid_idx >= bid_levels.len() {
            break;
        }

        let ask_price = ask_levels[ask_idx].0 * (1.0 + entry_fee);
        let bid_price = bid_levels[bid_idx].0 * (1.0 - exit_fee);

        if ask_price >= bid_price {
            break;
        }

        let fill = ask_remaining.min(bid_remaining);
        if fill <= 0.0 {
            break;
        }

        total_cost += fill * ask_price;

        ask_remaining -= fill;
        bid_remaining -= fill;

        if ask_remaining <= f64::EPSILON {
            ask_idx += 1;
            if ask_idx < ask_levels.len() {
                ask_remaining = ask_levels[ask_idx].1;
            }
        }
        if bid_remaining <= f64::EPSILON {
            bid_idx += 1;
            if bid_idx < bid_levels.len() {
                bid_remaining = bid_levels[bid_idx].1;
            }
        }
    }

    // Residual spread = best remaining ask - best remaining bid after walking
    let residual = if ask_idx < ask_levels.len() && bid_idx < bid_levels.len() {
        let remaining_ask = ask_levels[ask_idx].0;
        let remaining_bid = bid_levels[bid_idx].0;
        remaining_ask - remaining_bid
    } else {
        0.0
    };

    (total_cost, residual.max(0.0))
}
