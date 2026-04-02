# bellwether-maker

A prediction market arb logger and baby market maker built in Rust. Part of the Bellwether research infrastructure at Stanford GSB and the Hoover Institution.

## What This Is

This project scans cross-platform prediction markets for price discrepancies between Kalshi and Polymarket, simulates round-trip trades by walking the live order book, and logs hypothetical trades with full P&L detail. No real execution — yet.

It is the first step toward agentic trading infrastructure: the same order book walking logic that closes arb gaps today will eventually power directional AI agent trades against actively maintained markets.

## Why It Exists

Bellwether maintains a forecasting benchmark that evaluates AI agents against live prediction market prices. That benchmark is only scientifically credible if the prices it evaluates against are trustworthy. This bot actively maintains prices on a small set of important cross-platform markets by correcting gaps between Kalshi and Polymarket — producing a defensible ground truth that a passive aggregate cannot provide.

## Architecture

Two async Tokio tasks run concurrently, one per platform. Each maintains a local BTreeMap-based order book via WebSocket delta streams. The scorer fires on every book update (not on a timer) and logs opportunity windows with millisecond precision.

```
tokio::spawn → kalshi_ws_task    ─┐
                                   ├─→ Arc<RwLock<SharedBookState>> ─→ scorer ─→ opportunity_log
tokio::spawn → polymarket_ws_task ─┘
```

## Data Flow

Bellwether API (api.bellwethermetrics.com)
  → matched market list (is_matched: true)
  → BWR ticker, kalshi_ticker, polymarket_token_id per market

Native Kalshi WebSocket (wss://trading-api.kalshi.com/trade-api/v2/ws)
  → orderbook_snapshot on subscribe
  → orderbook_delta for incremental updates
  → JWT auth required (KALSHI_EMAIL / KALSHI_PASSWORD env vars)

Native Polymarket WebSocket (wss://ws-subscriptions-clob.polymarket.com/ws/market)
  → book snapshot on subscribe
  → price_change for incremental updates (absolute sizes, not deltas)
  → public, no auth required

Native REST APIs (Kalshi + Polymarket)
  → resolution date only, fetched once per market, cached permanently in state.json
  → fee rate check (Polymarket GET /fee-rate)

Do not use Bellwether for order book data — native WebSocket feeds provide full depth and real-time updates. Bellwether is for market discovery only.

## Key Files

- `arb_logger/src/main.rs` — async run loop, spawns WS tasks
- `arb_logger/src/ws.rs` — Kalshi and Polymarket WebSocket tasks, connection management, opportunity detection
- `arb_logger/src/api.rs` — Bellwether market discovery, resolution date fetch, fee rates
- `arb_logger/src/orderbook.rs` — BTreeMap-based order book walking and round-trip simulation
- `arb_logger/src/scorer.rs` — P&L computation, annualized returns, per-market executability analysis
- `arb_logger/src/state.rs` — state.json atomic persistence
- `arb_logger/src/report.rs` — report.md generation (opportunity windows + trade log)
- `arb_logger/src/types.rs` — all shared structs (PlatformBook, SharedBookState, OpportunityWindow, etc.)
- `state.json` — persisted market cache and trade log
- `report.md` — latest session output

## Running

```bash
export KALSHI_EMAIL="..."
export KALSHI_PASSWORD="..."
cd arb_logger

# Run with log capture (recommended for overnight runs)
caffeinate -i cargo run 2>&1 | tee arb_logger.log

# Monitor live in a second terminal
tail -f arb_logger.log

# Monitor only opportunity events (filter out heartbeats/INFO)
tail -f arb_logger.log | grep -E "OPEN|CLOSE|WARN"
```

`caffeinate -i` prevents macOS from sleeping while the process runs. Always use this for overnight runs — without it the MacBook may sleep and drop both WebSocket connections.

Runs continuously via WebSocket. Writes report.md every 30s. Heartbeat logged every 60s. Press Ctrl+C to stop.

## Structured Log Format

All log output goes to stderr via `eprintln!`. Format:
```
[HH:MM:SS] LEVEL TICKER event_description key=value key=value ...
```

Level labels are fixed-width 5 chars: `INFO `, `WARN `, `OPEN `, `CLOSE`.

## Infrastructure & Overhead

| Item                     | Cost/month |
|--------------------------|------------|
| Kalshi WebSocket         | ~$100      |
| Polymarket WebSocket     | $0         |
| Hosting (runs locally)   | $0         |
| Bellwether API           | $0         |
| **Total**                | **~$100**  |

Do not fall back to REST polling for order book data. The Kalshi WebSocket subscription is in place and is the correct architecture.

On a $5,000 capital deployment, $100/month overhead = 2.4% annual hurdle rate from infrastructure alone.

## Future Direction

- Real execution on Kalshi (REST/FIX) and Polymarket (CLOB + Polygon wallet)
- Flash loan atomic execution on Polymarket for zero-capital arb
- Directional forecasting agent replacing the mechanical arb logic
