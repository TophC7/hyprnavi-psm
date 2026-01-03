//! Edge detection for determining when to switch workspaces/monitors.
//!
//! Two detection modes are available:
//! - **Pixel mode** (default): Checks if window is at monitor boundary
//! - **Position mode** (`-p`): Checks if window is the extreme in that direction

use std::collections::HashMap;

use hyprland::{
    data::{Client, Monitor},
    dispatch::Direction,
    keyword::Keyword,
};

use crate::workspace::WorkspaceInfo;

/// Check if client is at edge using position-based detection.
///
/// Returns true if this window is the extreme window in the given direction
/// (e.g., leftmost window when direction is Left). This mode is useful for
/// scrolling layouts like hyprscroller where windows can be off-screen.
pub fn is_at_edge_position(
    client: &Client,
    ws_info: &HashMap<i32, WorkspaceInfo>,
    direction: &Direction,
) -> bool {
    let Some(info) = ws_info.get(&client.workspace.id) else {
        // No workspace info means we're alone - treat as edge
        return true;
    };

    let extreme = match direction {
        Direction::Left => &info.leftmost,
        Direction::Right => &info.rightmost,
        Direction::Up => &info.topmost,
        Direction::Down => &info.bottommost,
    };

    // We're at edge if we ARE the extreme window (or there is none)
    extreme
        .as_ref()
        .map(|a| *a == client.address)
        .unwrap_or(true)
}

/// Check if client is at edge using pixel-based detection.
///
/// Returns true if the window is physically at the monitor boundary,
/// accounting for gaps and reserved areas (bars, etc.). This is the
/// default mode, suitable for traditional tiled layouts.
pub fn is_at_edge_pixel(
    client: &Client,
    monitor: &Monitor,
    direction: &Direction,
    bordersize: i32,
) -> bool {
    let gap = get_gaps_out();

    // Client position and size
    let (cx, cy) = (client.at.0 as i32, client.at.1 as i32);
    let (cw, ch) = (client.size.0 as i32, client.size.1 as i32);

    // Monitor position and size
    let (mx, my) = (monitor.x, monitor.y);
    let (mw, mh) = (monitor.width as i32, monitor.height as i32);

    // Reserved areas (bars, etc.): Hyprland order is (top, bottom, right, left)
    let reserved = &monitor.reserved;

    // Check if window edge is within bordersize tolerance of monitor edge
    match direction {
        Direction::Left => (cx - (mx + reserved.3 as i32 + gap)).abs() <= bordersize,
        Direction::Right => ((cx + cw) - (mx + mw - reserved.2 as i32 - gap)).abs() <= bordersize,
        Direction::Up => (cy - (my + reserved.0 as i32 + gap)).abs() <= bordersize,
        Direction::Down => ((cy + ch) - (my + mh - reserved.1 as i32 - gap)).abs() <= bordersize,
    }
}

/// Get the outer gap size from Hyprland config.
fn get_gaps_out() -> i32 {
    Keyword::get("general:gaps_out")
        .ok()
        .and_then(|v| match v.value {
            hyprland::keyword::OptionValue::Int(i) => Some(i as i32),
            hyprland::keyword::OptionValue::Float(f) => Some(f as i32),
            _ => None,
        })
        .unwrap_or(0)
}
