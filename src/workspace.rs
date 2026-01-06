//! Pre-computes workspace metadata (extreme positions, adjacency) in O(n)
//! for O(1) lookups during navigation.

use std::collections::HashMap;

use hyprland::{data::Clients, shared::Address};

/// Extreme window positions and adjacent workspace IDs for a workspace.
#[derive(Default)]
pub struct WorkspaceInfo {
    pub leftmost: Option<Address>,
    pub rightmost: Option<Address>,
    pub topmost: Option<Address>,
    pub bottommost: Option<Address>,
    /// Wraps around to last workspace.
    pub prev_ws: i32,
    /// Wraps around to first workspace.
    pub next_ws: i32,
}

/// Excludes floating windows and special workspaces.
pub fn compute_workspace_info(clients: &Clients) -> HashMap<i32, WorkspaceInfo> {
    let mut info: HashMap<i32, WorkspaceInfo> = HashMap::new();
    let mut coords: HashMap<i32, (i16, i16, i16, i16)> = HashMap::new(); // (min_x, max_x, min_y, max_y)

    for client in clients.iter() {
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

    // Compute prev/next with wrap-around
    let mut ws_ids: Vec<i32> = info.keys().copied().collect();
    ws_ids.sort_unstable();
    let len = ws_ids.len();

    for (i, &ws) in ws_ids.iter().enumerate() {
        if let Some(entry) = info.get_mut(&ws) {
            entry.prev_ws = ws_ids[(i + len - 1) % len];
            entry.next_ws = ws_ids[(i + 1) % len];
        }
    }

    info
}
