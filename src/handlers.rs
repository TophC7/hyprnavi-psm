//! Navigation handlers for different scenarios.
//!
//! Handles focus movement, window swapping, and floating windows with
//! plugin-aware dispatching for split-monitor-workspaces integration.

use std::collections::HashMap;

use anyhow::Context;
use hyprland::{
    data::{Client, Monitor, Monitors},
    dispatch::{Direction, Dispatch, DispatchType, WindowIdentifier},
    shared::HyprData,
};

use crate::{
    cli::{Command, NavArgs},
    plugin::PluginState,
    workspace::WorkspaceInfo,
};

/// Handle navigation when no window is focused (empty workspace).
pub fn handle_empty_ws(cmd: &Command, plugins: &PluginState) -> anyhow::Result<()> {
    let args = cmd.args();

    if args.monitor {
        dispatch("focusmonitor", dir_char(&cmd.direction()))
    } else if plugins.split_monitor_workspaces {
        // Plugin handles workspace-per-monitor natively
        dispatch(
            "split-workspace",
            if cmd.is_forward() { "+1" } else { "-1" },
        )
    } else {
        dispatch("workspace", if cmd.is_forward() { "e+1" } else { "e-1" })
    }
}

/// Handle navigation for floating windows.
pub fn handle_floating(
    args: &NavArgs,
    direction: &Direction,
    is_at_edge: bool,
) -> anyhow::Result<()> {
    if args.swap {
        // Move floating window
        if is_at_edge && args.monitor {
            dispatch("movewindow", &format!("mon:{}", dir_char(direction)))?;
            dispatch("centerwindow", "")
        } else {
            dispatch("movewindow", dir_char(direction))
        }
    } else {
        // Just move focus
        Dispatch::call(DispatchType::MoveFocus(direction.clone())).context("Failed to move focus")
    }
}

/// Handle swap/move mode for tiled windows.
///
/// When position mode (`-p`) is enabled with hyprscrolling, uses `layoutmsg movewindowto`
/// for proper column-aware window movement instead of swapping.
pub fn handle_swap(
    args: &NavArgs,
    direction: &Direction,
    is_at_edge: bool,
    is_alone_in_column: bool,
    plugins: &PluginState,
    current_monitor: &Monitor,
) -> anyhow::Result<()> {
    // Scroller mode
    if args.position && plugins.hyprscrolling {
        let is_vertical = matches!(direction, Direction::Up | Direction::Down);
        let is_forward = matches!(direction, Direction::Right | Direction::Down);

        // At edge: move to monitor (l/r) or workspace (u/d)
        if is_at_edge {
            if is_vertical {
                // Move to adjacent workspace
                let delta = if is_forward { "+1" } else { "-1" };
                return if plugins.split_monitor_workspaces {
                    dispatch("split-movetoworkspace", delta)
                } else {
                    dispatch("movetoworkspace", &format!("e{}", delta))
                };
            } else if args.monitor
                && is_alone_in_column
                && has_monitor_in_direction(current_monitor, direction)
            {
                // Only move to monitor if already alone in column (can't promote further)
                return dispatch("movewindow", &format!("mon:{}", dir_char(direction)));
            }
        }

        // Column movement/promotion (prioritized over monitor movement)
        return dispatch(
            "layoutmsg",
            &format!("movewindowto {}", dir_char(direction)),
        );
    }

    // Standard mode
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

/// Check if a monitor exists in the given direction from the current monitor.
///
/// Uses simple position comparison (m.x < cx for left, etc.) which handles
/// rotated monitors correctly since it doesn't depend on reported width/height.
fn has_monitor_in_direction(current: &Monitor, direction: &Direction) -> bool {
    let Ok(monitors) = Monitors::get() else {
        return false;
    };

    let (cx, cy) = (current.x, current.y);

    monitors.iter().any(|m| {
        if m.id == current.id {
            return false;
        }

        match direction {
            Direction::Left => m.x < cx,
            Direction::Right => m.x > cx,
            Direction::Up => m.y < cy,
            Direction::Down => m.y > cy,
        }
    })
}

/// Handle focus movement for tiled windows.
///
/// At edge, switches to adjacent workspace (or monitor with `-m` flag).
/// Uses split-monitor-workspaces plugin dispatchers when available.
pub fn handle_focus(
    cmd: &Command,
    client: &Client,
    ws_info: &HashMap<i32, WorkspaceInfo>,
    is_at_edge: bool,
    plugins: &PluginState,
) -> anyhow::Result<()> {
    let args = cmd.args();
    let direction = cmd.direction();

    // Not at edge - move focus
    if !is_at_edge {
        return if args.position && plugins.hyprscrolling {
            // Scroller mode: use layoutmsg for proper scrolling focus
            dispatch("layoutmsg", &format!("focus {}", dir_char(&direction)))
        } else {
            Dispatch::call(DispatchType::MoveFocus(direction)).context("Failed to move focus")
        };
    }

    // At edge with monitor flag - switch monitors
    if args.monitor {
        return dispatch("focusmonitor", dir_char(&direction));
    }

    // At edge - switch to adjacent workspace
    if plugins.split_monitor_workspaces {
        // Plugin handles workspace-per-monitor and wrapping natively
        return dispatch(
            "split-workspace",
            if cmd.is_forward() { "+1" } else { "-1" },
        );
    }

    // Fallback: manual workspace navigation with wrapping logic
    let Some(current) = ws_info.get(&client.workspace.id) else {
        return dispatch("workspace", if cmd.is_forward() { "e+1" } else { "e-1" });
    };

    let current_ws = client.workspace.id;
    let target_ws = if cmd.is_forward() {
        current.next_ws
    } else {
        current.prev_ws
    };

    // Check for wrap condition
    let would_wrap = if cmd.is_forward() {
        target_ws < current_ws
    } else {
        target_ws > current_ws
    };

    if args.no_wrap && would_wrap {
        return Ok(());
    }

    // Don't navigate if we'd stay on the same workspace (single workspace case)
    if target_ws == current_ws {
        return Ok(());
    }

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

// --- Helpers ---

/// Convert direction to single-character string for dispatcher args.
fn dir_char(d: &Direction) -> &'static str {
    match d {
        Direction::Up => "u",
        Direction::Down => "d",
        Direction::Left => "l",
        Direction::Right => "r",
    }
}

/// Call a custom Hyprland dispatcher.
fn dispatch(cmd: &str, arg: &str) -> anyhow::Result<()> {
    Dispatch::call(DispatchType::Custom(cmd, arg))
        .with_context(|| format!("Failed: {} {}", cmd, arg))
}
