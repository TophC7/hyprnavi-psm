//! Plugin detection with per-session caching.
//!
//! Detects if split-monitor-workspaces plugin is active and caches the result
//! using Hyprland's instance signature to avoid repeated checks.

use std::process::Command;

const SPLIT_WS_PLUGIN: &str = "split-monitor-workspaces";

/// Cached plugin state for the current Hyprland session.
///
/// Uses `$HYPRLAND_INSTANCE_SIGNATURE` to create a unique cache file per session.
/// The cache is stored in `/run/user/$UID/` (tmpfs) for fast access.
pub struct PluginState {
    pub split_monitor_workspaces: bool,
}

impl PluginState {
    /// Detect active plugins, using cache when available.
    ///
    /// First call per Hyprland session queries `hyprctl plugin list` and caches
    /// the result. Subsequent calls read from cache (~0.1ms vs ~5ms).
    pub fn detect() -> Self {
        Self {
            split_monitor_workspaces: is_split_ws_active(),
        }
    }
}

/// Check if split-monitor-workspaces plugin is active, with caching.
///
/// Cache stores "1" or "0" for simple boolean lookup.
fn is_split_ws_active() -> bool {
    if let Some(cache_path) = get_cache_path() {
        // Try cache first
        if let Ok(cached) = std::fs::read_to_string(&cache_path) {
            return cached.trim() == "1";
        }

        // Cache miss - query and store
        let active = query_has_plugin(SPLIT_WS_PLUGIN);
        let _ = std::fs::write(&cache_path, if active { "1" } else { "0" });
        return active;
    }

    // No cache path available - query directly
    query_has_plugin(SPLIT_WS_PLUGIN)
}

/// Get the cache file path based on Hyprland instance signature.
fn get_cache_path() -> Option<String> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    Some(format!("{}/hyprnavi-splitws-{}", runtime_dir, sig))
}

/// Query hyprctl to check if a specific plugin is loaded.
///
/// Uses simple string matching on JSON output to avoid serde_json dependency.
fn query_has_plugin(name: &str) -> bool {
    Command::new("hyprctl")
        .args(["-j", "plugin", "list"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|json| {
            // Check both with and without space after colon
            json.contains(&format!("\"name\": \"{}\"", name))
                || json.contains(&format!("\"name\":\"{}\"", name))
        })
        .unwrap_or(false)
}
