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
pub struct AppState {
    pub markets: HashMap<String, MarketState>,
    pub trades: Vec<TradeRecord>,
    #[serde(default)]
    pub executability: Vec<MarketExecutability>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            markets: HashMap::new(),
            trades: Vec::new(),
            executability: Vec::new(),
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
    pub polymarket_fee: f64,
}

impl Default for FeeRates {
    fn default() -> Self {
        Self {
            kalshi_fee: 0.0,
            polymarket_fee: 0.0,
        }
    }
}
