//! Plugin detection with per-session caching.
//!
//! Detects Hyprland plugins and caches results using the instance signature
//! to avoid repeated hyprctl queries.

use std::process::Command;

/// Plugin names we care about.
const SPLIT_WS: &str = "split-monitor-workspaces";
const SCROLLER: &str = "hyprscrolling";

/// Detected plugin state for the current Hyprland session.
///
/// Cached in `/run/user/$UID/` (tmpfs) per `$HYPRLAND_INSTANCE_SIGNATURE`.
pub struct PluginState {
    /// split-monitor-workspaces: per-monitor workspace navigation.
    pub split_monitor_workspaces: bool,
    /// hyprscrolling: scrolling layout with column-based window movement.
    pub hyprscrolling: bool,
}

impl PluginState {
    /// Detect active plugins, using cache when available.
    ///
    /// First call per Hyprland session queries `hyprctl plugin list` and caches
    /// the result. Subsequent calls read from cache (~0.1ms vs ~5ms).
    pub fn detect() -> Self {
        if let Some(cache_path) = get_cache_path() {
            // Try cache first
            if let Ok(cached) = std::fs::read_to_string(&cache_path) {
                return Self::parse_cache(&cached);
            }

            // Cache miss - query and store
            let json = query_plugins();
            let state = Self::from_json(&json);
            let _ = std::fs::write(&cache_path, state.to_cache());
            return state;
        }

        // No cache path - query directly
        Self::from_json(&query_plugins())
    }

    fn from_json(json: &str) -> Self {
        Self {
            split_monitor_workspaces: json_has_plugin(json, SPLIT_WS),
            hyprscrolling: json_has_plugin(json, SCROLLER),
        }
    }

    fn parse_cache(cached: &str) -> Self {
        let mut split_ws = false;
        let mut scroller = false;

        for part in cached.split(',') {
            match part.trim() {
                "splitws" => split_ws = true,
                "scroller" => scroller = true,
                _ => {}
            }
        }

        Self {
            split_monitor_workspaces: split_ws,
            hyprscrolling: scroller,
        }
    }

    fn to_cache(&self) -> String {
        let mut parts = Vec::new();
        if self.split_monitor_workspaces {
            parts.push("splitws");
        }
        if self.hyprscrolling {
            parts.push("scroller");
        }
        parts.join(",")
    }
}

/// Get cache file path based on Hyprland instance signature.
fn get_cache_path() -> Option<String> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    Some(format!("{}/hyprnavi-{}", runtime_dir, sig))
}

/// Query hyprctl for plugin list JSON.
fn query_plugins() -> String {
    Command::new("hyprctl")
        .args(["-j", "plugin", "list"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
}

/// Check if JSON contains a specific plugin name.
fn json_has_plugin(json: &str, name: &str) -> bool {
    json.contains(&format!("\"name\": \"{}\"", name))
        || json.contains(&format!("\"name\":\"{}\"", name))
}
