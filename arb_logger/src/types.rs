use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use tokio::time::Instant;

// ---------------------------------------------------------------------------
// WebSocket shared state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PlatformBook {
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    pub last_updated: Instant,
    pub seq: Option<u64>,
    pub last_message_latency_ms: Option<i64>,
    pub avg_message_latency_ms: Option<f64>,
    latency_samples: Vec<i64>,
}

impl PlatformBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_updated: Instant::now(),
            seq: None,
            last_message_latency_ms: None,
            avg_message_latency_ms: None,
            latency_samples: Vec::with_capacity(100),
        }
    }

    /// Record a message latency sample and update rolling average (last 100).
    pub fn record_latency(&mut self, latency_ms: i64) {
        self.last_message_latency_ms = Some(latency_ms);
        if self.latency_samples.len() >= 100 {
            self.latency_samples.remove(0);
        }
        self.latency_samples.push(latency_ms);
        let sum: i64 = self.latency_samples.iter().sum();
        self.avg_message_latency_ms = Some(sum as f64 / self.latency_samples.len() as f64);
    }
}

pub struct SharedBookState {
    pub kalshi: HashMap<String, PlatformBook>,
    pub polymarket: HashMap<String, PlatformBook>,
}

impl SharedBookState {
    pub fn new() -> Self {
        Self {
            kalshi: HashMap::new(),
            polymarket: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Execution overhead (measured once at startup)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOverhead {
    pub kalshi_construction_ms: u64,
    pub polymarket_construction_ms: u64,
    pub kalshi_network_ms: u64,
    pub polymarket_network_ms: u64,
    pub polygon_settlement_ms: u64,
    pub vpn_overhead_ms: u64, // from VPN_OVERHEAD_MS env var, default 0
}

impl ExecutionOverhead {
    pub fn total_floor_ms(&self) -> u64 {
        self.polygon_settlement_ms
            + self.polymarket_network_ms
            + self.kalshi_network_ms
            + self.polymarket_construction_ms
            + self.kalshi_construction_ms
            + self.vpn_overhead_ms
    }
}

// ---------------------------------------------------------------------------
// Market executability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketExecutability {
    pub ticker: String,
    pub window_count: u32,
    pub avg_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub execution_floor_ms: f64,
    pub executable: bool,
    pub confidence: String,
    pub avg_message_latency_ms: f64,
}

// ---------------------------------------------------------------------------
// Detailed round-trip and leg structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegDetail {
    pub platform: String,
    pub side: String,
    pub action: String,
    pub shares: f64,
    pub avg_price: f64,
    pub total_cost: f64,
    pub levels_consumed: u32,
    pub book_snapshot: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundTripDetail {
    pub kalshi_yes_asks: Vec<(f64, f64)>,
    pub kalshi_no_asks: Vec<(f64, f64)>,
    pub polymarket_yes_asks: Vec<(f64, f64)>,
    pub polymarket_no_asks: Vec<(f64, f64)>,

    pub direction: String,

    pub entry_leg: LegDetail,
    pub exit_leg: LegDetail,

    pub gross_spread: f64,
    pub fees: f64,
    pub net_profit: f64,
    pub nev_per_share: f64,

    pub cost_to_profitably_standardize: f64,
    pub cost_to_fully_standardize: f64,
    pub residual_spread_after_trade: f64,
    pub fully_standardized: bool,
}

// ---------------------------------------------------------------------------
// Opportunity window
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityWindow {
    pub ticker: String,
    pub opened_at_ms: u64,
    pub closed_at_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub peak_nev: f64,
    pub peak_spread: f64,
    pub current_nev: f64,
    pub current_spread: f64,
    pub current_depth: f64,
    pub current_direction: String,
    pub update_count: u32,
    pub best_round_trip: Option<RoundTrip>,
    pub round_trip: Option<RoundTripDetail>,
}

// ---------------------------------------------------------------------------
// Market mapping
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MarketMapping {
    pub bwr_ticker: String,
    pub kalshi_ticker: String,
    pub polymarket_token: String,
}

// ---------------------------------------------------------------------------
// Trade logging types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillEvent {
    pub price: f64,
    pub size: f64,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundTrip {
    pub direction: String,
    pub entry_platform: String,
    pub exit_platform: String,
    pub entry_fills: Vec<FillEvent>,
    pub exit_fills: Vec<FillEvent>,
    pub total_entry_cost: f64,
    pub total_exit_proceeds: f64,
    pub shares_filled: f64,
    pub shares_exited: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub ticker: String,
    pub logged_at: String,
    pub resolution_date: Option<String>,
    pub days_left: Option<i64>,
    pub direction: String,
    pub entry_fills: Vec<FillEvent>,
    pub avg_entry_price: f64,
    pub total_entry_cost: f64,
    pub exit_fills: Vec<FillEvent>,
    pub avg_exit_price: f64,
    pub total_exit_proceeds: f64,
    pub shares_filled: f64,
    pub shares_exited: f64,
    pub exit_type: String,
    pub gross_profit: f64,
    pub fees: f64,
    pub net_profit: f64,
    pub nev_per_share: f64,
    pub ann_return: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub resolution_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub ticker: String,
    pub executed_at: String,
    pub direction: String,
    pub shares: f64,
    pub entry_platform: String,
    pub entry_side: String,
    pub entry_limit_price: f64,
    pub entry_status: String,
    pub entry_response: String,
    pub exit_platform: String,
    pub exit_side: String,
    pub exit_limit_price: f64,
    pub exit_status: String,
    pub exit_response: String,
    pub total_capital: f64,
    pub expected_profit: f64,
    pub expected_nev: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub markets: HashMap<String, MarketState>,
    pub trades: Vec<TradeRecord>,
    #[serde(default)]
    pub executability: Vec<MarketExecutability>,
    #[serde(default)]
    pub executions: Vec<ExecutionRecord>,
    #[serde(default)]
    pub simulator: SimulatorState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            markets: HashMap::new(),
            trades: Vec::new(),
            executability: Vec::new(),
            executions: Vec::new(),
            simulator: SimulatorState::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchedMarket {
    pub ticker: String,
    pub kalshi_ticker: Option<String>,
    pub polymarket_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FeeRates {
    pub kalshi_fee: f64,
    /// Polymarket base fee rate per token (key = token_id, value = base rate e.g. 0.04).
    /// Actual fee per share = base_rate * price * (1 - price).
    pub polymarket_fee_rates: std::collections::HashMap<String, f64>,
}

impl FeeRates {
    /// Get the Polymarket base fee rate for a given token.
    pub fn pm_rate(&self, token_id: &str) -> f64 {
        self.polymarket_fee_rates.get(token_id).copied().unwrap_or(0.0)
    }
}

impl Default for FeeRates {
    fn default() -> Self {
        Self {
            kalshi_fee: 0.0,
            polymarket_fee_rates: std::collections::HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Paper trading simulator
// ---------------------------------------------------------------------------

/// A simulated position held to resolution. Never closed early.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperPosition {
    pub ticker: String,
    pub opened_at: String,              // RFC 3339
    pub hour_utc: u32,                  // 0-23, for time-of-day analysis
    pub direction: String,              // e.g. "BUY_YES_POLYMARKET_YES_KALSHI"
    pub entry_platform: String,
    pub exit_platform: String,
    pub shares: f64,
    pub entry_cost: f64,                // total capital locked on entry leg
    pub exit_proceeds: f64,             // expected proceeds when market resolves
    pub net_profit: f64,                // exit_proceeds - entry_cost (arb spread captured)
    pub nev_per_share: f64,
    pub gross_spread: f64,              // cross-platform spread before fees
    pub kalshi_internal_spread: f64,    // best_ask - best_bid on Kalshi
    pub pm_internal_spread: f64,        // best_ask - best_bid on Polymarket
    pub kalshi_depth: f64,              // total $ at profitable prices on Kalshi side
    pub pm_depth: f64,                  // total $ at profitable prices on Polymarket side
    pub window_age_ms: u64,             // how long the window had been open when we entered
    pub resolution_date: Option<String>,
    pub days_to_resolution: Option<i64>,
}

/// Per-market re-entry statistics observed during this session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketReentryStats {
    pub ticker: String,
    pub entry_count: u32,               // how many times we've "executed" on this market
    pub first_entry_at: String,         // RFC 3339
    pub last_entry_at: String,          // RFC 3339
    pub observation_hours: f64,         // wall-clock hours since first entry
    pub avg_profit_per_entry: f64,
    pub avg_capital_per_entry: f64,
}

/// Top-level simulator state, persisted in state.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorState {
    /// All paper positions (held to resolution).
    pub positions: Vec<PaperPosition>,

    /// Per-market re-entry tracking.
    pub reentry_stats: HashMap<String, MarketReentryStats>,

    /// Peak simultaneous capital across all open positions (observed).
    pub peak_capital: f64,

    /// Current total capital deployed.
    pub current_capital: f64,

    /// Cumulative arb profit (sum of net_profit across all positions).
    pub total_arb_profit: f64,

    /// Platform APY rates (fetched/configured at startup).
    pub kalshi_apy: f64,
    pub polymarket_apy: f64,

    /// Session start time (RFC 3339) for observation window calculations.
    pub session_started_at: String,
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            reentry_stats: HashMap::new(),
            peak_capital: 0.0,
            current_capital: 0.0,
            total_arb_profit: 0.0,
            kalshi_apy: 0.035,      // 3.5% — Kalshi current rate
            polymarket_apy: 0.04,   // 4.0% — Polymarket current rate
            session_started_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
