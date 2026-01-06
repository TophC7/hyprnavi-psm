//! Plugin detection with per-session caching.
//!
//! Detects Hyprland plugins and caches results using the instance signature
//! to avoid repeated hyprctl queries.

use std::process::Command;

const SPLIT_WS: &str = "split-monitor-workspaces";
const SCROLLER: &str = "hyprscrolling";

/// Detected plugin state, cached per `$HYPRLAND_INSTANCE_SIGNATURE`.
pub struct PluginState {
    pub split_monitor_workspaces: bool,
    pub hyprscrolling: bool,
}

impl PluginState {
    /// Queries `hyprctl plugin list`, caching result for subsequent calls.
    pub fn detect() -> Self {
        if let Some(cache_path) = get_cache_path() {
            if let Ok(cached) = std::fs::read_to_string(&cache_path) {
                return Self::parse_cache(&cached);
            }
            let json = query_plugins();
            let state = Self::from_json(&json);
            let _ = std::fs::write(&cache_path, state.to_cache());
            return state;
        }
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

fn get_cache_path() -> Option<String> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    Some(format!("{}/hyprnavi-{}", runtime_dir, sig))
}

fn query_plugins() -> String {
    Command::new("hyprctl")
        .args(["-j", "plugin", "list"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
}

fn json_has_plugin(json: &str, name: &str) -> bool {
    json.contains(&format!("\"name\": \"{}\"", name))
        || json.contains(&format!("\"name\":\"{}\"", name))
}
