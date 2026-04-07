use crate::types::SharedBookState;
use serde_json::json;

const BOOKS_FILE: &str = "../books.json";

/// Write a snapshot of all order books to books.json.
/// Called every 30s from the periodic tick.
pub async fn write_book_snapshot(state: &tokio::sync::RwLock<SharedBookState>) {
    let books = state.read().await;

    let mut kalshi_out = serde_json::Map::new();
    for (ticker, book) in &books.kalshi {
        let bids: Vec<_> = book
            .bids
            .iter()
            .rev()
            .map(|(p, s)| json!({"price": p.into_inner(), "size": s}))
            .collect();
        let asks: Vec<_> = book
            .asks
            .iter()
            .map(|(p, s)| json!({"price": p.into_inner(), "size": s}))
            .collect();
        kalshi_out.insert(
            ticker.clone(),
            json!({
                "bids": bids,
                "asks": asks,
                "bid_levels": bids.len(),
                "ask_levels": asks.len(),
                "last_updated_secs_ago": book.last_updated.elapsed().as_secs(),
            }),
        );
    }

    let mut pm_out = serde_json::Map::new();
    for (token, book) in &books.polymarket {
        let bids: Vec<_> = book
            .bids
            .iter()
            .rev()
            .map(|(p, s)| json!({"price": p.into_inner(), "size": s}))
            .collect();
        let asks: Vec<_> = book
            .asks
            .iter()
            .map(|(p, s)| json!({"price": p.into_inner(), "size": s}))
            .collect();
        pm_out.insert(
            token.clone(),
            json!({
                "bids": bids,
                "asks": asks,
                "bid_levels": bids.len(),
                "ask_levels": asks.len(),
                "last_updated_secs_ago": book.last_updated.elapsed().as_secs(),
            }),
        );
    }

    let snapshot = json!({
        "snapshot_at": chrono::Utc::now().to_rfc3339(),
        "kalshi_markets": kalshi_out.len(),
        "polymarket_markets": pm_out.len(),
        "kalshi": kalshi_out,
        "polymarket": pm_out,
    });

    if let Err(e) = std::fs::write(BOOKS_FILE, serde_json::to_string_pretty(&snapshot).unwrap_or_default()) {
        eprintln!(
            "[{}] WARN  SYSTEM books_write_failed error={}",
            chrono::Local::now().format("%H:%M:%S"),
            e,
        );
    }
}
