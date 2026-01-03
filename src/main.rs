use std::collections::HashMap;

use anyhow::Context;
use clap::Parser;
use hyprland::{
    data::{Client, Clients, Monitor},
    dispatch::{Direction, Dispatch, DispatchType, WindowIdentifier},
    keyword::Keyword,
    shared::{Address, HyprData, HyprDataActive, HyprDataActiveOptional},
};

use crate::cli::{Cli, Command, NavArgs};

mod cli;

/// Pre-computed workspace metadata for O(1) lookups.
#[derive(Default)]
struct WorkspaceInfo {
    leftmost: Option<Address>,
    rightmost: Option<Address>,
    topmost: Option<Address>,
    bottommost: Option<Address>,
    prev_ws: i32,
    next_ws: i32,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cmd = &cli.command;
    let args = cmd.args();
    let direction = cmd.direction();

    // Handle empty workspace case
    let Some(active_client) = Client::get_active().context("Failed to query active window")? else {
        return handle_empty_ws(cmd);
    };

    let all_clients = Clients::get().context("Failed to get window list")?;
    let active_monitor = Monitor::get_active().context("Failed to get active monitor")?;
    let ws_info = compute_workspace_info(&all_clients);

    // Determine if we're at the edge
    let is_at_edge = if args.position {
        is_at_edge_position(&active_client, &ws_info, &direction)
    } else {
        is_at_edge_pixel(&active_client, &active_monitor, &direction, args.bordersize)
    };

    // Dispatch to appropriate handler
    if active_client.floating {
        handle_floating(args, &direction, is_at_edge)
    } else if args.swap {
        handle_swap(args, &direction, is_at_edge)
    } else {
        handle_focus(cmd, &active_client, &ws_info, is_at_edge)
    }
}

// --- Handlers ---

fn handle_empty_ws(cmd: &Command) -> anyhow::Result<()> {
    let args = cmd.args();
    if args.monitor {
        dispatch("focusmonitor", dir_char(&cmd.direction()))
    } else {
        dispatch("workspace", if cmd.is_forward() { "e+1" } else { "e-1" })
    }
}

fn handle_floating(args: &NavArgs, direction: &Direction, is_at_edge: bool) -> anyhow::Result<()> {
    if args.swap {
        if is_at_edge && args.monitor {
            dispatch("movewindow", &format!("mon:{}", dir_char(direction)))?;
            dispatch("centerwindow", "")
        } else {
            dispatch("movewindow", dir_char(direction))
        }
    } else {
        Dispatch::call(DispatchType::MoveFocus(direction.clone())).context("Failed to move focus")
    }
}

fn handle_swap(args: &NavArgs, direction: &Direction, is_at_edge: bool) -> anyhow::Result<()> {
    if is_at_edge {
        if args.monitor {
            dispatch("movewindow", &format!("mon:{}", dir_char(direction)))
        } else {
            dispatch("movewindow", dir_char(direction))
        }
    } else {
        Dispatch::call(DispatchType::SwapWindow(direction.clone())).context("Failed to swap")
    }
}

fn handle_focus(
    cmd: &Command,
    client: &Client,
    ws_info: &HashMap<i32, WorkspaceInfo>,
    is_at_edge: bool,
) -> anyhow::Result<()> {
    let args = cmd.args();
    let direction = cmd.direction();

    if !is_at_edge {
        return Dispatch::call(DispatchType::MoveFocus(direction)).context("Failed to move focus");
    }

    if args.monitor {
        return dispatch("focusmonitor", dir_char(&direction));
    }

    // Switch to adjacent workspace
    let Some(current) = ws_info.get(&client.workspace.id) else {
        return dispatch("workspace", if cmd.is_forward() { "e+1" } else { "e-1" });
    };

    let target_ws = if cmd.is_forward() {
        current.next_ws
    } else {
        current.prev_ws
    };

    // Try to focus window at opposite edge on target workspace
    if let Some(target) = ws_info.get(&target_ws) {
        let addr = match direction {
            Direction::Right => target.leftmost.as_ref(),
            Direction::Left => target.rightmost.as_ref(),
            Direction::Down => target.topmost.as_ref(),
            Direction::Up => target.bottommost.as_ref(),
        };
        if let Some(addr) = addr {
            return Dispatch::call(DispatchType::FocusWindow(WindowIdentifier::Address(
                addr.clone(),
            )))
            .context("Failed to focus window");
        }
    }

    dispatch("workspace", &target_ws.to_string())
}

// --- Pre-computation ---

fn compute_workspace_info(clients: &Clients) -> HashMap<i32, WorkspaceInfo> {
    let mut info: HashMap<i32, WorkspaceInfo> = HashMap::new();
    // Track coordinates alongside addresses for comparison
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

    // Compute prev/next workspace relationships
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

// --- Edge Detection ---

fn is_at_edge_position(
    client: &Client,
    ws_info: &HashMap<i32, WorkspaceInfo>,
    direction: &Direction,
) -> bool {
    let Some(info) = ws_info.get(&client.workspace.id) else {
        return true;
    };

    let extreme = match direction {
        Direction::Left => &info.leftmost,
        Direction::Right => &info.rightmost,
        Direction::Up => &info.topmost,
        Direction::Down => &info.bottommost,
    };
    extreme
        .as_ref()
        .map(|a| *a == client.address)
        .unwrap_or(true)
}

fn is_at_edge_pixel(
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
    let (r0, r1, r2, r3) = (
        monitor.reserved.0 as i32,
        monitor.reserved.1 as i32,
        monitor.reserved.2 as i32,
        monitor.reserved.3 as i32,
    );

    match direction {
        Direction::Left => (cx - (mx + r3 + gap)).abs() <= bordersize,
        Direction::Right => ((cx + cw) - (mx + mw - r2 - gap)).abs() <= bordersize,
        Direction::Up => (cy - (my + r0 + gap)).abs() <= bordersize,
        Direction::Down => ((cy + ch) - (my + mh - r1 - gap)).abs() <= bordersize,
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

// --- Helpers ---

fn dir_char(d: &Direction) -> &'static str {
    match d {
        Direction::Up => "u",
        Direction::Down => "d",
        Direction::Left => "l",
        Direction::Right => "r",
    }
}

fn dispatch(cmd: &str, arg: &str) -> anyhow::Result<()> {
    Dispatch::call(DispatchType::Custom(cmd, arg))
        .with_context(|| format!("Failed: {} {}", cmd, arg))
}
