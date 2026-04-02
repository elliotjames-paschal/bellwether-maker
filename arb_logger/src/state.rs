use crate::types::AppState;
use std::path::Path;

const STATE_FILE: &str = "state.json";
const STATE_TMP: &str = "state.json.tmp";

/// Load state from state.json. Returns default if file doesn't exist or is corrupt.
pub fn load_state() -> AppState {
    let path = Path::new(STATE_FILE);
    if !path.exists() {
        return AppState::default();
    }

    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Warning: failed to parse state.json ({}), starting fresh", e);
            AppState::default()
        }),
        Err(e) => {
            eprintln!("Warning: failed to read state.json ({}), starting fresh", e);
            AppState::default()
        }
    }
}

/// Save state to state.json atomically (write to .tmp, then rename).
pub fn save_state(state: &AppState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize state: {}", e))?;

    std::fs::write(STATE_TMP, &json)
        .map_err(|e| format!("Failed to write {}: {}", STATE_TMP, e))?;

    std::fs::rename(STATE_TMP, STATE_FILE)
        .map_err(|e| format!("Failed to rename {} to {}: {}", STATE_TMP, STATE_FILE, e))?;

    Ok(())
}
