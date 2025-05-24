use std::cmp;

use hyprland::{
    data::{Client, Clients},
    dispatch::{Direction, Dispatch, DispatchType, WindowIdentifier},
    shared::{HyprData, HyprDataActiveOptional},
};

use crate::cli::{Command, Flags};

mod cli;

fn main() -> anyhow::Result<()> {
    let params: Flags = argh::from_env();

    // 2. Criar o slice &[Client] a partir do wrapper
    let all_clients = &Clients::get()?;

    let Some(act_client) = Client::get_active().ok().flatten() else {
        // No active window, likely an empty workspace.
        // Original behavior: navigate workspaces. Swap flag is not directly applicable here.
        return handle_in_empty_ws(params);
    };

    let act_ws_id = act_client.workspace.id;

    // Get the leftmost and rightmost clients on the *active* workspace.
    // This expect assumes that if act_client exists and is non-special,
    // get_bound_client will successfully find bounds on its workspace.
    let (ws_left_client, ws_right_client) = get_bound_client(all_clients, act_ws_id)
        .expect("Failed to get bound clients for the active workspace. The active window might be 'special' or not found in the client list.");

    use Command::*;
    match params.cmd {
        Next(cmd_params) => {
            // Command 'r' (right)
            if cmd_params.swap {
                if is_bound(&act_client, ws_right_client, true) {
                    // Active window is the rightmost on its workspace: move active window to monitor to the right.
                    Dispatch::call(DispatchType::Custom("movewindow", "r"))?;
                } else {
                    // Not the rightmost: swap with window to the right on the same workspace.
                    Dispatch::call(DispatchType::SwapWindow(Direction::Right))?;
                }
            } else {
                // Not swapping, original focus logic.
                if is_bound(&act_client, ws_right_client, true) {
                    // Active window is rightmost on its workspace.
                    // Determine the next workspace and a client on it to focus.

                    let fallback_client =
                        all_clients.iter().as_slice().first().ok_or_else(|| {
                            anyhow::anyhow!(
                                "No clients found, cannot determine fallback focus target."
                            )
                        })?;

                    let (_prev_ws_id, next_ws_id) =
                        get_neighborhood_workspace(all_clients, act_ws_id);

                    let target_client =
                        get_bound_client(all_clients, next_ws_id) // Get leftmost client on next_ws_id
                            .map_or(fallback_client, |(c, _)| c); // Or fallback if next_ws_id is empty/special

                    Dispatch::call(DispatchType::FocusWindow(WindowIdentifier::Address(
                        target_client.address.clone(),
                    )))?;
                } else {
                    // Focus window to the right on the same workspace.
                    Dispatch::call(DispatchType::MoveFocus(Direction::Right))?;
                }
            }
        }
        Prev(cmd_params) => {
            // Command 'l' (left)
            if cmd_params.swap {
                if is_bound(&act_client, ws_left_client, false) {
                    // Active window is the leftmost on its workspace: move active window to monitor to the left.
                    Dispatch::call(DispatchType::Custom("movewindow", "l"))?;
                } else {
                    // Not the leftmost: swap with window to the left on the same workspace.
                    Dispatch::call(DispatchType::SwapWindow(Direction::Left))?;
                }
            } else {
                // Not swapping, original focus logic.
                if is_bound(&act_client, ws_left_client, false) {
                    // Active window is leftmost on its workspace.
                    let fallback_client =
                        all_clients.iter().as_slice().last().ok_or_else(|| {
                            anyhow::anyhow!(
                                "No clients found, cannot determine fallback focus target."
                            )
                        })?;

                    let (prev_ws_id, _next_ws_id) =
                        get_neighborhood_workspace(all_clients, act_ws_id);

                    let target_client =
                        get_bound_client(all_clients, prev_ws_id) // Get rightmost client on prev_ws_id
                            .map_or(fallback_client, |(_, c)| c); // Or fallback if prev_ws_id is empty/special

                    Dispatch::call(DispatchType::FocusWindow(WindowIdentifier::Address(
                        target_client.address.clone(),
                    )))?;
                } else {
                    // Focus window to the left on the same workspace.
                    Dispatch::call(DispatchType::MoveFocus(Direction::Left))?;
                }
            }
        }
    };

    Ok(())
}

// Handles navigation when the current workspace is empty.
fn handle_in_empty_ws(params: Flags) -> anyhow::Result<()> {
    use Command::*;
    // Dispatch to change active workspace.
    Dispatch::call(match params.cmd {
        Next(_) => DispatchType::Custom("workspace", "e+1"),
        Prev(_) => DispatchType::Custom("workspace", "e-1"),
    })?;
    Ok(())
}

// Determines the previous and next workspace IDs relative to the active one.
// This is the original implementation from the provided code.
#[inline]
fn get_neighborhood_workspace(clients: &Clients, act_ws_id: i32) -> (i32, i32) {
    let near_than_last = |id_cur: i32, id_last: i32| -> bool {
        let dist_cur = (id_cur - act_ws_id).abs();
        let dist_last = (id_last - act_ws_id).abs();
        dist_cur != 0 && (dist_last == 0 || dist_last > dist_cur)
    };

    let (prev, next, max, min) = clients
        .iter()
        .filter(|client| {
            client.workspace.id != act_ws_id && !client.workspace.name.starts_with("special")
        })
        .fold(
            (act_ws_id, act_ws_id, act_ws_id, act_ws_id),
            |acc, client| {
                let id = client.workspace.id;
                let prev_ws = if act_ws_id > id && near_than_last(id, acc.0) {
                    id
                } else {
                    acc.0
                };
                let next_ws = if act_ws_id < id && near_than_last(id, acc.1) {
                    id
                } else {
                    acc.1
                };
                (prev_ws, next_ws, cmp::max(acc.2, id), cmp::min(acc.3, id))
            },
        );

    let mut final_prev = if prev == act_ws_id { max } else { prev };
    let mut final_next = if next == act_ws_id { min } else { next };

    // If no other non-special workspaces exist, prev/next should be act_ws_id.
    let other_non_special_workspaces_exist = clients
        .iter()
        .any(|c| c.workspace.id != act_ws_id && !c.workspace.name.starts_with("special"));

    if !other_non_special_workspaces_exist {
        final_prev = act_ws_id;
        final_next = act_ws_id;
    }

    (final_prev, final_next)
}

// Gets the leftmost and rightmost non-special clients on a given workspace.
fn get_bound_client(all_clients: &Clients, workspace_id: i32) -> Option<(&Client, &Client)> {
    let ws_clients: Vec<&Client> = all_clients
        .iter()
        .filter(|client| {
            client.workspace.id == workspace_id && !client.workspace.name.starts_with("special")
        })
        .collect();

    if ws_clients.is_empty() {
        return None;
    }

    // .unwrap() is safe here because ws_clients is confirmed not empty.
    let left_client = ws_clients.iter().min_by_key(|c| c.at.0).unwrap();
    let right_client = ws_clients.iter().max_by_key(|c| c.at.0 + c.size.0).unwrap();

    Some((*left_client, *right_client))
}

// Checks if the active client is at a boundary (leftmost or rightmost) of its workspace.
// Modified slightly for clarity with bound_client_candidate.
#[inline]
fn is_bound(act: &Client, bound_client_candidate: &Client, is_checking_right_bound: bool) -> bool {
    // If active client *is* the candidate (e.g., it's the only client on the workspace meeting criteria).
    if act.address == bound_client_candidate.address {
        return true;
    }

    // Check if the relevant edges align.
    if is_checking_right_bound {
        // Is active client's right edge at the same X-coordinate as the candidate rightmost client's right edge?
        (act.at.0 + act.size.0) == (bound_client_candidate.at.0 + bound_client_candidate.size.0)
    } else {
        // Is active client's left edge at the same X-coordinate as the candidate leftmost client's left edge?
        act.at.0 == bound_client_candidate.at.0
    }
}
