//! Edge detection for determining when to switch workspaces/monitors.
//!
//! Three detection modes are available:
//! - **Pixel mode** (default): Checks if window is at monitor boundary
//! - **Position mode** (`-p`): Checks if window is the extreme in that direction
//! - **Scroller mode** (`-p` + hyprscrolling): Column-aware edge detection

use std::collections::HashMap;

use hyprland::{
    data::{Client, Clients, Monitor},
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

/// Check if client is at edge for hyprscrolling layouts.
///
/// For l/r: checks if in leftmost/rightmost column (workspace-level).
/// For u/d: checks if at top/bottom of current column (column-level).
///
/// In hyprscrolling, windows in the same column share the same x position.
pub fn is_at_edge_scroller(client: &Client, clients: &Clients, direction: &Direction) -> bool {
    let my_x = client.at.0;
    let my_y = client.at.1;
    let my_ws = client.workspace.id;

    clients.iter().all(|c| {
        if c.workspace.id != my_ws || c.floating {
            return true;
        }
        match direction {
            // l/r: no column further in that direction
            Direction::Left => c.at.0 >= my_x,
            Direction::Right => c.at.0 <= my_x,
            // u/d: no window further in our column (same x)
            Direction::Up => c.at.0 != my_x || c.at.1 >= my_y,
            Direction::Down => c.at.0 != my_x || c.at.1 <= my_y,
        }
    })
}

/// Returns true if no other window shares this column (same x position).
/// Used to decide if window can promote further or should move to monitor.
pub fn is_alone_in_column(client: &Client, clients: &Clients) -> bool {
    let my_x = client.at.0;
    let my_ws = client.workspace.id;
    let my_addr = &client.address;

    !clients
        .iter()
        .any(|c| c.workspace.id == my_ws && !c.floating && c.at.0 == my_x && c.address != *my_addr)
}

/// Pixel-based edge detection (default mode). Accounts for gaps and reserved areas.
pub fn is_at_edge_pixel(
    client: &Client,
    monitor: &Monitor,
    direction: &Direction,
    bordersize: i32,
) -> bool {
    let gap = get_gaps_out();
    let (cx, cy) = (client.at.0 as i32, client.at.1 as i32);
    let (cw, ch) = (client.size.0 as i32, client.size.1 as i32);
    let (mx, my) = (monitor.x, monitor.y);
    let (mw, mh) = (monitor.width as i32, monitor.height as i32);
    // Hyprland reserved order: (top, bottom, right, left)
    let reserved = &monitor.reserved;

    match direction {
        Direction::Left => (cx - (mx + reserved.3 as i32 + gap)).abs() <= bordersize,
        Direction::Right => ((cx + cw) - (mx + mw - reserved.2 as i32 - gap)).abs() <= bordersize,
        Direction::Up => (cy - (my + reserved.0 as i32 + gap)).abs() <= bordersize,
        Direction::Down => ((cy + ch) - (my + mh - reserved.1 as i32 - gap)).abs() <= bordersize,
    }
}

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
