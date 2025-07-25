use hyprland::{
    data::{Client, Clients, Monitor},
    dispatch::{Direction, Dispatch, DispatchType, WindowIdentifier},
    keyword::Keyword,
    shared::{HyprData, HyprDataActive, HyprDataActiveOptional},
};

// Import definitions from the command-line interface (CLI).
// `argh` populates these structs with the arguments passed to the program.
use crate::cli::{Command, Flags};

// Declare the `cli` module, which should exist in the `cli.rs` file.
mod cli;

// --- Main Function ---
// Program entry point.
fn main() -> anyhow::Result<()> {
    // 1. Parse the command-line arguments (e.g., "r", "l -s").
    let params: Flags = argh::from_env();
    // 2. Get the list of all open windows from Hyprland.
    let all_clients = Clients::get()?;

    // 3. Try to get the active window. If there isn't one...
    let Some(active_client) = Client::get_active().ok().flatten() else {
        // ... it means the workspace is empty. Call the function to handle this case.
        return handle_in_empty_ws(&params.cmd);
    };

    // 4. If there is an active window, get information about the current monitor.
    let active_monitor = Monitor::get_active()?;

    // 5. Dispatch execution based on the provided subcommand (r, l, u, d).
    //    It extracts parameters from each command (like `p.swap` and `p.bordersize`)
    //    and passes them to the appropriate handler function.
    match params.cmd {
        Command::Up(p) => handle_vertical_nav(Direction::Up, p.swap, &active_client)?,
        Command::Down(p) => handle_vertical_nav(Direction::Down, p.swap, &active_client)?,
        Command::Left(p) => handle_horizontal_nav(
            Direction::Left,
            p.swap,
            p.bordersize,
            &active_client,
            &active_monitor,
            &all_clients,
        )?,
        Command::Right(p) => handle_horizontal_nav(
            Direction::Right,
            p.swap,
            p.bordersize,
            &active_client,
            &active_monitor,
            &all_clients,
        )?,
    };

    Ok(())
}

// --- Handler Functions ---

/// Handles navigation when there are no windows on the current workspace.
fn handle_in_empty_ws(command: &Command) -> anyhow::Result<()> {
    // Determines whether to go to the previous or next workspace based on the command.
    // "e+1" / "e-1" are Hyprland commands for navigating to the next/previous existing workspace.
    let direction = match command {
        Command::Right(_) | Command::Up(_) => "e+1",
        Command::Left(_) | Command::Down(_) => "e-1",
    };
    Dispatch::call(DispatchType::Custom("workspace", direction))?;
    Ok(())
}

/// Handles vertical navigation (Up/Down), which has simpler logic.
fn handle_vertical_nav(
    direction: Direction,
    swap: bool,
    active_client: &Client,
) -> anyhow::Result<()> {
    if swap {
        // For floating windows, swapping doesn't make sense, so we move the window instead.
        if active_client.floating {
            let dir_char = match direction {
                Direction::Up => "u",
                Direction::Down => "d",
                _ => unreachable!(), // This function only handles Up/Down.
            };
            Dispatch::call(DispatchType::Custom("movewindow", dir_char))?;
        } else {
            // For tiled windows, we use the native swap command.
            Dispatch::call(DispatchType::SwapWindow(direction))?;
        }
    } else {
        // If not swapping, just move the focus.
        Dispatch::call(DispatchType::MoveFocus(direction))?;
    }
    Ok(())
}

/// Handles horizontal navigation (Left/Right), which has complex "wrapping" logic.
fn handle_horizontal_nav(
    direction: Direction,
    swap: bool,
    bordersize: Option<i32>,
    active_client: &Client,
    active_monitor: &Monitor,
    all_clients: &Clients,
) -> anyhow::Result<()> {
    // Determines if we are checking the right or left screen boundary.
    let is_checking_right_bound = match direction {
        Direction::Right => true,
        Direction::Left => false,
        _ => unreachable!(),
    };

    // `is_bound` checks if the active window is physically at the monitor's edge.
    let is_at_boundary = is_bound(
        active_client,
        active_monitor,
        bordersize.unwrap_or(0),
        is_checking_right_bound,
    );

    // Specific logic block for floating windows.
    if active_client.floating {
        if swap {
            let dir_char = match direction {
                Direction::Right => "r",
                Direction::Left => "l",
                _ => unreachable!(),
            };
            // If at the boundary, move the window to the next MONITOR.
            if is_at_boundary {
                Dispatch::call(DispatchType::Custom(
                    "movewindow",
                    &format!("mon:{}", dir_char),
                ))?;
                // Center the window on the new monitor for better placement.
                Dispatch::call(DispatchType::Custom("centerwindow", ""))?;
            } else {
                // If not at the boundary, just move the window in the specified direction.
                Dispatch::call(DispatchType::Custom("movewindow", dir_char))?;
            }
        } else {
            // Focusing floating windows is simple, using the Hyprland default.
            Dispatch::call(DispatchType::MoveFocus(direction))?;
        }
        return Ok(());
    }

    // Logic block for tiled windows.
    if swap {
        if is_at_boundary {
            // If at the boundary, move the window to the adjacent workspace (which could be on another monitor).
            let dir_char = match direction {
                Direction::Right => "r",
                Direction::Left => "l",
                _ => unreachable!(),
            };
            Dispatch::call(DispatchType::Custom("movewindow", dir_char))?;
        } else {
            // Otherwise, just swap with the neighboring window on the same workspace.
            Dispatch::call(DispatchType::SwapWindow(direction))?;
        }
    } else {
        // Focus logic
        if is_at_boundary {
            // At the boundary: the magic happens. We move the focus to the adjacent workspace.
            let (prev_ws, next_ws) =
                find_adjacent_workspaces(all_clients, active_client.workspace.id);
            let (target_ws_id, find_rightmost) = match direction {
                // If moving right, we focus the leftmost client of the next workspace.
                Direction::Right => (next_ws, false),
                // If moving left, we focus the rightmost client of the previous workspace.
                Direction::Left => (prev_ws, true),
                _ => unreachable!(),
            };
            // Tries to find a target client on the destination workspace.
            if let Some((l_client, r_client)) = get_bound_client(all_clients, target_ws_id, false) {
                let target_client = if find_rightmost { r_client } else { l_client };
                Dispatch::call(DispatchType::FocusWindow(WindowIdentifier::Address(
                    target_client.address.clone(),
                )))?;
            } else {
                // If the destination workspace is empty, just switch to it.
                Dispatch::call(DispatchType::Custom("workspace", &target_ws_id.to_string()))?;
            }
        } else {
            // If not at the boundary, just move the focus to the neighboring window.
            Dispatch::call(DispatchType::MoveFocus(direction))?;
        }
    }
    Ok(())
}

// --- Helper Functions ---

/// Checks if a window is physically at the edge of the monitor.
/// This function is crucial as it considers gaps and reserved areas (status bars).
#[inline]
fn is_bound(
    act: &Client,
    monitor: &Monitor,
    bordersize: i32,
    is_checking_right_bound: bool,
) -> bool {
    // Gets the `gaps_out` value from Hyprland settings.
    let gaps_out = match Keyword::get("general:gaps_out") {
        Ok(value) => match value.value {
            hyprland::keyword::OptionValue::Int(v) => v as i32,
            hyprland::keyword::OptionValue::Float(v) => v as i32,
            _ => 0,
        },
        Err(_) => 0,
    };
    // Calculates the exact X-coordinates of the usable area's left and right edges.
    let mon_right = monitor.x + monitor.width as i32 - monitor.reserved.2 as i32 - gaps_out;
    let mon_left = monitor.x + monitor.reserved.3 as i32 + gaps_out;

    // Gets the X-coordinates of the active window.
    let act_right = (act.at.0 + act.size.0) as i32;
    let act_left = act.at.0 as i32;

    // Compares the window edge with the monitor edge, with a tolerance (`bordersize`).
    if is_checking_right_bound {
        (act_right - mon_right).abs() <= bordersize
    } else {
        (act_left - mon_left).abs() <= bordersize
    }
}

/// Finds the IDs of the previous and next workspaces in a circular manner.
fn find_adjacent_workspaces(clients: &Clients, active_ws_id: i32) -> (i32, i32) {
    // Uses a `BTreeSet` to get a sorted, unique list of workspace IDs.
    let mut ws_ids: std::collections::BTreeSet<i32> = clients
        .iter()
        .filter(|c| c.workspace.id > 0)
        .map(|c| c.workspace.id)
        .collect();
    // Ensures the active workspace is in the list (important if it's empty).
    ws_ids.insert(active_ws_id);

    let sorted_ids: Vec<i32> = ws_ids.into_iter().collect();
    if sorted_ids.len() <= 1 {
        // If there's only one workspace, the previous and next are itself.
        return (active_ws_id, active_ws_id);
    }
    // Finds the position of the current workspace in the sorted list.
    let current_pos = sorted_ids
        .iter()
        .position(|&id| id == active_ws_id)
        .unwrap_or(0);

    // Uses modulo arithmetic to find the previous and next indices, creating a "wrapping" effect.
    let prev_idx = (current_pos + sorted_ids.len() - 1) % sorted_ids.len();
    let next_idx = (current_pos + 1) % sorted_ids.len();
    (sorted_ids[prev_idx], sorted_ids[next_idx])
}

/// Finds the leftmost and rightmost clients on a given workspace.
/// Used to determine which window to focus when "jumping" from one workspace to another.
fn get_bound_client<'a>(
    all_clients: &'a Clients,
    workspace_id: i32,
    floating: bool,
) -> Option<(&'a Client, &'a Client)> {
    let ws_clients: Vec<&Client> = all_clients
        .iter()
        .filter(|c| {
            c.workspace.id == workspace_id
                && !c.workspace.name.starts_with("special")
                && c.floating == floating
        })
        .collect();

    if ws_clients.is_empty() {
        return None;
    }

    // Finds the client with the smallest X-coordinate (leftmost) and the largest (rightmost).
    let left_client = ws_clients.iter().min_by_key(|c| c.at.0)?;
    let right_client = ws_clients.iter().max_by_key(|c| c.at.0)?;
    Some((left_client, right_client))
}

