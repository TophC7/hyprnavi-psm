//! Workspace metadata pre-computation for efficient navigation.
//!
//! Computes extreme window positions and workspace relationships in a single
//! O(n) pass over all clients, enabling O(1) lookups during navigation.

use std::collections::HashMap;

use hyprland::{data::Clients, shared::Address};

/// Pre-computed workspace metadata for O(1) lookups.
///
/// Stores the addresses of extreme windows (leftmost, rightmost, etc.) and
/// adjacent workspace relationships for efficient cross-workspace navigation.
#[derive(Default)]
pub struct WorkspaceInfo {
    /// Address of the leftmost window in this workspace.
    pub leftmost: Option<Address>,
    /// Address of the rightmost window in this workspace.
    pub rightmost: Option<Address>,
    /// Address of the topmost window in this workspace.
    pub topmost: Option<Address>,
    /// Address of the bottommost window in this workspace.
    pub bottommost: Option<Address>,
    /// Previous workspace ID (wraps around).
    pub prev_ws: i32,
    /// Next workspace ID (wraps around).
    pub next_ws: i32,
}

/// Compute workspace metadata for all active workspaces.
///
/// Performs a single O(n) pass over all clients to determine:
/// - Extreme window positions (leftmost, rightmost, topmost, bottommost)
/// - Workspace adjacency relationships (prev/next)
///
/// Floating windows and special workspaces are excluded.
pub fn compute_workspace_info(clients: &Clients) -> HashMap<i32, WorkspaceInfo> {
    let mut info: HashMap<i32, WorkspaceInfo> = HashMap::new();
    // Track coordinates alongside addresses for comparison
    // Format: (min_x, max_x, min_y, max_y)
    let mut coords: HashMap<i32, (i16, i16, i16, i16)> = HashMap::new();

    for client in clients.iter() {
        // Skip floating windows, special workspaces, and invalid workspace IDs
        if client.workspace.id <= 0
            || client.floating
            || client.workspace.name.starts_with("special")
        {
            continue;
        }

        let ws = client.workspace.id;
        let (x, y) = (client.at.0, client.at.1);
        let (x2, y2) = (x + client.size.0, y + client.size.1);

        let entry = info.entry(ws).or_default();
        let coord = coords
            .entry(ws)
            .or_insert((i16::MAX, i16::MIN, i16::MAX, i16::MIN));

        // Update extreme positions if this window is more extreme
        if x < coord.0 {
            coord.0 = x;
            entry.leftmost = Some(client.address.clone());
        }
        if x2 > coord.1 {
            coord.1 = x2;
            entry.rightmost = Some(client.address.clone());
        }
        if y < coord.2 {
            coord.2 = y;
            entry.topmost = Some(client.address.clone());
        }
        if y2 > coord.3 {
            coord.3 = y2;
            entry.bottommost = Some(client.address.clone());
        }
    }

    // Compute prev/next workspace relationships (sorted order with wrap-around)
    let mut ws_ids: Vec<i32> = info.keys().copied().collect();
    ws_ids.sort_unstable();
    let len = ws_ids.len();

    for (i, &ws) in ws_ids.iter().enumerate() {
        if let Some(entry) = info.get_mut(&ws) {
            // Wrap around: first workspace's prev is last, last's next is first
            entry.prev_ws = ws_ids[(i + len - 1) % len];
            entry.next_ws = ws_ids[(i + 1) % len];
        }
    }

    info
}
