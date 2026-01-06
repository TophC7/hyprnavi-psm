//! Navigation handlers with plugin-aware dispatching.

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

/// Empty workspace: go to monitor or adjacent workspace.
pub fn handle_empty_ws(cmd: &Command, plugins: &PluginState) -> anyhow::Result<()> {
    let args = cmd.args();

    if args.monitor {
        dispatch("focusmonitor", dir_char(&cmd.direction()))
    } else if plugins.split_monitor_workspaces {
        dispatch(
            "split-workspace",
            if cmd.is_forward() { "+1" } else { "-1" },
        )
    } else {
        dispatch("workspace", if cmd.is_forward() { "e+1" } else { "e-1" })
    }
}

/// Floating windows: move window (-s) or focus.
pub fn handle_floating(
    args: &NavArgs,
    direction: &Direction,
    is_at_edge: bool,
) -> anyhow::Result<()> {
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

/// Swap/move tiled windows. With `-ps` + hyprscrolling: column-aware movement.
pub fn handle_swap(
    args: &NavArgs,
    direction: &Direction,
    is_at_edge: bool,
    is_alone_in_column: bool,
    plugins: &PluginState,
    current_monitor: &Monitor,
) -> anyhow::Result<()> {
    // Scroller mode: prioritize column promotion over monitor movement
    if args.position && plugins.hyprscrolling {
        let is_vertical = matches!(direction, Direction::Up | Direction::Down);
        let is_forward = matches!(direction, Direction::Right | Direction::Down);

        if is_at_edge {
            if is_vertical {
                // u/d at edge: move to adjacent workspace
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
                // l/r at edge + alone in column: move to monitor
                return dispatch("movewindow", &format!("mon:{}", dir_char(direction)));
            }
        }

        // Default: promote/move within columns
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

/// Simple position check that handles rotated monitors correctly.
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

/// Focus movement for tiled windows. At edge: workspace or monitor (-m).
pub fn handle_focus(
    cmd: &Command,
    client: &Client,
    ws_info: &HashMap<i32, WorkspaceInfo>,
    is_at_edge: bool,
    plugins: &PluginState,
) -> anyhow::Result<()> {
    let args = cmd.args();
    let direction = cmd.direction();

    if !is_at_edge {
        return if args.position && plugins.hyprscrolling {
            dispatch("layoutmsg", &format!("focus {}", dir_char(&direction)))
        } else {
            Dispatch::call(DispatchType::MoveFocus(direction)).context("Failed to move focus")
        };
    }

    if args.monitor {
        return dispatch("focusmonitor", dir_char(&direction));
    }

    if plugins.split_monitor_workspaces {
        return dispatch(
            "split-workspace",
            if cmd.is_forward() { "+1" } else { "-1" },
        );
    }

    // Manual workspace navigation with wrapping
    let Some(current) = ws_info.get(&client.workspace.id) else {
        return dispatch("workspace", if cmd.is_forward() { "e+1" } else { "e-1" });
    };

    let current_ws = client.workspace.id;
    let target_ws = if cmd.is_forward() {
        current.next_ws
    } else {
        current.prev_ws
    };

    let would_wrap = if cmd.is_forward() {
        target_ws < current_ws
    } else {
        target_ws > current_ws
    };

    if args.no_wrap && would_wrap {
        return Ok(());
    }

    if target_ws == current_ws {
        return Ok(());
    }

    // Focus window at opposite edge on target workspace
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
