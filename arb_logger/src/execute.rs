use crate::{api, audit, kalshi_rest, orderbook, polymarket_auth, state};
use crate::types::ExecutionRecord;
use chrono::Local;
use ethers_signers::Signer;
use reqwest::Client;
use rsa::pkcs1::DecodeRsaPrivateKey;
use std::io::{self, Write};

const DEFAULT_MAX_USD: f64 = 500.0;

/// Main execution entry point. Fetches books, simulates trade, and optionally executes.
pub async fn run_execute(bwr_ticker: &str, dry_run: bool) -> Result<(), String> {
    // Parse --max-capital from args (default $500)
    let args: Vec<String> = std::env::args().collect();
    let max_capital = args
        .iter()
        .position(|a| a == "--max-capital")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(DEFAULT_MAX_USD);
    let client = Client::new();

    // 1. Resolve ticker → kalshi_ticker + polymarket_token
    eprintln!("[EXEC] Resolving ticker {}...", bwr_ticker);
    let markets = api::get_tracked_markets(&client).await?;
    let market = markets
        .iter()
        .find(|m| m.ticker == bwr_ticker)
        .ok_or_else(|| {
            let available: Vec<&str> = markets.iter().map(|m| m.ticker.as_str()).collect();
            format!(
                "Ticker '{}' not found in active markets. Available:\n  {}",
                bwr_ticker,
                available.join("\n  ")
            )
        })?;

    let kalshi_ticker = market
        .kalshi_ticker
        .as_ref()
        .ok_or("No Kalshi ticker mapping for this market")?;
    let pm_yes_token = market
        .polymarket_token
        .as_ref()
        .ok_or("No Polymarket token mapping for this market")?;

    eprintln!(
        "[EXEC] Mapped: kalshi={} polymarket={}...{}",
        kalshi_ticker,
        &pm_yes_token[..8.min(pm_yes_token.len())],
        &pm_yes_token[pm_yes_token.len().saturating_sub(4)..],
    );

    // 2. Fetch fee rate + neg_risk flag
    let pm_fee_rates = api::fetch_polymarket_fee_rates(&client, &[pm_yes_token.clone()]).await;
    let pm_fee = *pm_fee_rates.get(pm_yes_token.as_str()).unwrap_or(&0.0);
    let fee_rate_bps = (pm_fee * 10000.0).round() as u64;
    let neg_risk = polymarket_auth::is_neg_risk(&client, pm_yes_token).await;

    eprintln!(
        "[EXEC] Fees: kalshi=0% polymarket={:.1}% ({} bps) neg_risk={}",
        pm_fee * 100.0,
        fee_rate_bps,
        neg_risk,
    );

    // 3. Fetch both orderbooks via REST
    eprintln!("[EXEC] Fetching orderbooks...");
    let (k_result, pm_result) = tokio::join!(
        audit::fetch_kalshi_book(&client, kalshi_ticker),
        audit::fetch_polymarket_book(&client, pm_yes_token),
    );
    let k_book = k_result?;
    let pm_book = pm_result?;

    eprintln!(
        "[EXEC] Books: kalshi_bids={} kalshi_asks={} pm_bids={} pm_asks={}",
        k_book.bids.len(),
        k_book.asks.len(),
        pm_book.bids.len(),
        pm_book.asks.len(),
    );

    // 4. Simulate round trip
    let (rt, detail) =
        orderbook::simulate_round_trip_btree(&k_book, &pm_book, 0.0, pm_fee)
            .ok_or("No profitable round-trip found at current book state")?;

    let shares = rt.shares_filled;
    let profit = detail.net_profit;
    let nev = detail.nev_per_share;

    // Compute actual execution costs (both legs are BUY orders)
    // Entry leg: buying YES on entry_platform
    // Exit leg: buying NO on exit_platform (complement of YES sell)
    let no_cost = rt.shares_exited - rt.total_exit_proceeds;
    let total_capital = rt.total_entry_cost + no_cost;

    // 5. Safety check (after printing plan)
    let over_cap = total_capital > max_capital;

    // 6. Determine execution parameters for each leg
    let entry_is_pm = rt.entry_platform == "polymarket";

    // Entry: worst (highest) ask price from fills
    let entry_limit_price = rt
        .entry_fills
        .iter()
        .map(|f| f.price)
        .fold(0.0_f64, f64::max);

    // Exit: worst (lowest) YES bid price from fills
    let exit_worst_yes_price = rt
        .exit_fills
        .iter()
        .map(|f| f.price)
        .fold(f64::INFINITY, f64::min);
    let exit_no_price = 1.0 - exit_worst_yes_price;

    // 7. Print execution plan
    println!();
    println!(
        "=== EXECUTION PLAN: {} ===",
        bwr_ticker
    );
    println!();

    if entry_is_pm {
        println!("  LEG 1 — BUY YES on Polymarket");
        println!("    Token: {}...", &pm_yes_token[..20.min(pm_yes_token.len())]);
        println!(
            "    {:.0} shares @ ${:.2} = ${:.2}",
            shares, entry_limit_price, rt.total_entry_cost,
        );
        println!();
        println!("  LEG 2 — BUY NO on Kalshi");
        println!("    Ticker: {}", kalshi_ticker);
        println!(
            "    {:.0} shares @ ${:.2} = ${:.2}",
            rt.shares_exited, exit_no_price, no_cost,
        );
    } else {
        println!("  LEG 1 — BUY YES on Kalshi");
        println!("    Ticker: {}", kalshi_ticker);
        println!(
            "    {:.0} shares @ ${:.2} = ${:.2}",
            shares, entry_limit_price, rt.total_entry_cost,
        );
        println!();
        println!("  LEG 2 — BUY NO on Polymarket");
        println!("    Token: {}...", &pm_yes_token[..20.min(pm_yes_token.len())]);
        println!(
            "    {:.0} shares @ ${:.2} = ${:.2}",
            rt.shares_exited, exit_no_price, no_cost,
        );
    }

    println!();
    println!(
        "  Total capital: ${:.2}",
        total_capital,
    );
    println!(
        "  Expected profit: ${:.2} ({:.1}c/share NEV)",
        profit,
        nev * 100.0,
    );
    println!(
        "  Direction: {}",
        detail.direction,
    );
    println!();

    // 8. Capital safety check
    if over_cap {
        println!(
            "  *** BLOCKED: Total capital ${:.2} exceeds safety cap ${:.2} ***",
            total_capital, max_capital,
        );
        println!("  Use --max-capital {} to override.", total_capital.ceil() as u64);
        println!();
        return Ok(());
    }

    // 9. Dry run check
    if dry_run {
        println!("  (DRY RUN — no orders submitted)");
        println!();
        return Ok(());
    }

    // 9. Wait for user confirmation
    print!("  Proceed? [y/n] ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Read stdin: {}", e))?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("  Aborted.");
        return Ok(());
    }

    println!();
    eprintln!("[EXEC] Executing trades...");

    // 10. Load credentials
    // Kalshi
    let kalshi_api_key = std::env::var("KALSHI_API_KEY")
        .map_err(|_| "KALSHI_API_KEY env var not set")?;
    let kalshi_key_path = std::env::var("KALSHI_PRIVATE_KEY_PATH")
        .map_err(|_| "KALSHI_PRIVATE_KEY_PATH env var not set")?;
    let kalshi_pem = std::fs::read_to_string(&kalshi_key_path)
        .map_err(|e| format!("Read Kalshi key {}: {}", kalshi_key_path, e))?;
    let kalshi_private_key = rsa::RsaPrivateKey::from_pkcs1_pem(&kalshi_pem)
        .map_err(|e| format!("Parse Kalshi key: {}", e))?;

    // Polymarket
    let pm_key_hex = std::env::var("POLYMARKET_PRIVATE_KEY")
        .map_err(|_| "POLYMARKET_PRIVATE_KEY env var not set")?;
    let pm_wallet: ethers_signers::LocalWallet = pm_key_hex
        .parse()
        .map_err(|e| format!("Parse Polymarket wallet key: {}", e))?;
    let pm_wallet = pm_wallet.with_chain_id(137u64);
    let pm_creds = polymarket_auth::load_or_derive_creds(&client, &pm_wallet).await?;

    // 11. Execute both legs concurrently
    let kalshi_count = shares as u32;

    // Build Kalshi order params
    let (k_side, k_price_cents) = if entry_is_pm {
        // Kalshi is exit leg: BUY NO
        let cents = (exit_no_price * 100.0).ceil() as u32;
        ("no", cents)
    } else {
        // Kalshi is entry leg: BUY YES
        let cents = (entry_limit_price * 100.0).ceil() as u32;
        ("yes", cents)
    };

    // Build Polymarket order params
    let (pm_side, pm_price, pm_token) = if entry_is_pm {
        // Polymarket is entry leg: BUY YES
        (0u8, entry_limit_price, pm_yes_token.clone())
    } else {
        // Polymarket is exit leg: BUY NO
        let no_token = polymarket_auth::fetch_no_token(&client, pm_yes_token).await?;
        eprintln!("[EXEC] Fetched NO token: {}...{}", &no_token[..8.min(no_token.len())], &no_token[no_token.len().saturating_sub(4)..]);
        (0u8, exit_no_price, no_token)
    };

    let pm_size = shares;

    eprintln!(
        "[EXEC] Submitting: Kalshi {} {} {}c x{} | PM {} token={} ${:.4} x{:.0}",
        "buy", k_side, k_price_cents, kalshi_count,
        if pm_side == 0 { "BUY" } else { "SELL" },
        &pm_token[..12.min(pm_token.len())],
        pm_price,
        pm_size,
    );

    // Execute concurrently
    let kalshi_fut = kalshi_rest::submit_order(
        &client,
        &kalshi_api_key,
        &kalshi_private_key,
        kalshi_ticker,
        k_side,
        "buy",
        k_price_cents,
        kalshi_count,
    );

    let pm_fut = polymarket_auth::submit_order(
        &client,
        &pm_wallet,
        &pm_creds,
        &pm_token,
        pm_side,
        pm_price,
        pm_size,
        fee_rate_bps,
        neg_risk,
    );

    let (kalshi_result, pm_result) = tokio::join!(kalshi_fut, pm_fut);

    // 12. Report results
    println!("=== EXECUTION RESULTS ===");
    println!();

    let k_status;
    let k_response;
    match &kalshi_result {
        Ok(resp) => {
            k_status = "OK".to_string();
            k_response = serde_json::to_string(&resp.order).unwrap_or_default();
            println!("  Kalshi:      SUCCESS");
            if let Some(order) = &resp.order {
                println!("    {}", serde_json::to_string_pretty(order).unwrap_or_default());
            }
        }
        Err(e) => {
            k_status = "FAILED".to_string();
            k_response = e.clone();
            println!("  Kalshi:      FAILED — {}", e);
        }
    }

    let pm_status;
    let pm_response;
    match &pm_result {
        Ok(resp) => {
            pm_status = "OK".to_string();
            pm_response = format!("order_id={:?}", resp.order_id);
            println!("  Polymarket:  SUCCESS");
            if let Some(oid) = &resp.order_id {
                println!("    order_id: {}", oid);
            }
            if let Some(hashes) = &resp.transaction_hashes {
                for h in hashes {
                    println!("    tx: {}", h);
                }
            }
        }
        Err(e) => {
            pm_status = "FAILED".to_string();
            pm_response = e.clone();
            println!("  Polymarket:  FAILED — {}", e);
        }
    }

    // Warn if one leg failed
    if kalshi_result.is_ok() != pm_result.is_ok() {
        println!();
        println!(
            "  *** WARNING: One leg failed! You have a DIRECTIONAL position, not an arb. ***"
        );
        println!("  *** Check both platforms and unwind manually if needed. ***");
    }

    println!();

    // 13. Log execution to state.json
    let record = ExecutionRecord {
        ticker: bwr_ticker.to_string(),
        executed_at: Local::now().to_rfc3339(),
        direction: detail.direction.clone(),
        shares,
        entry_platform: rt.entry_platform.clone(),
        entry_side: if entry_is_pm { "YES".into() } else { "YES".into() },
        entry_limit_price: if entry_is_pm { entry_limit_price } else { entry_limit_price },
        entry_status: if entry_is_pm {
            pm_status.clone()
        } else {
            k_status.clone()
        },
        entry_response: if entry_is_pm {
            pm_response.clone()
        } else {
            k_response.clone()
        },
        exit_platform: rt.exit_platform.clone(),
        exit_side: "NO".into(),
        exit_limit_price: exit_no_price,
        exit_status: if entry_is_pm {
            k_status
        } else {
            pm_status
        },
        exit_response: if entry_is_pm {
            k_response
        } else {
            pm_response
        },
        total_capital,
        expected_profit: profit,
        expected_nev: nev,
    };

    let mut app_state = state::load_state();
    app_state.executions.push(record);
    state::save_state(&app_state)?;
    eprintln!("[EXEC] Execution logged to state.json");

    Ok(())
}
