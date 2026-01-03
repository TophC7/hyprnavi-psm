//! hyprnavi - Smart navigation tool for Hyprland with edge-detection.
//!
//! Navigate seamlessly between windows, workspaces, and monitors using
//! a unified interface. Supports both traditional tiled layouts and
//! scrolling layouts (hyprscroller), with automatic integration for
//! split-monitor-workspaces plugin.

use anyhow::Context;
use clap::Parser;
use hyprland::{
    data::{Client, Clients, Monitor},
    shared::{HyprData, HyprDataActive, HyprDataActiveOptional},
};

use crate::cli::Cli;

mod cli;
mod edge;
mod handlers;
mod plugin;
mod workspace;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cmd = &cli.command;
    let args = cmd.args();
    let direction = cmd.direction();

    // Detect active plugins (cached per Hyprland session)
    let plugins = plugin::PluginState::detect();

    // Handle empty workspace case (no focused window)
    let Some(active_client) = Client::get_active().context("Failed to query active window")? else {
        return handlers::handle_empty_ws(cmd, &plugins);
    };

    // Gather workspace and monitor data
    let all_clients = Clients::get().context("Failed to get window list")?;
    let active_monitor = Monitor::get_active().context("Failed to get active monitor")?;
    let ws_info = workspace::compute_workspace_info(&all_clients);

    // Determine if we're at the edge using selected detection mode
    let is_at_edge = if args.position && plugins.hyprscrolling {
        // Scroller mode: column-aware edge detection for u/d
        edge::is_at_edge_scroller(&active_client, &all_clients, &direction)
    } else if args.position {
        edge::is_at_edge_position(&active_client, &ws_info, &direction)
    } else {
        edge::is_at_edge_pixel(&active_client, &active_monitor, &direction, args.bordersize)
    };

    // Dispatch to appropriate handler based on window type and mode
    if active_client.floating {
        handlers::handle_floating(args, &direction, is_at_edge)
    } else if args.swap {
        // For scroller mode: check if window is alone in its column (can't promote further)
        let is_alone = edge::is_alone_in_column(&active_client, &all_clients);
        handlers::handle_swap(
            args,
            &direction,
            is_at_edge,
            is_alone,
            &plugins,
            &active_monitor,
        )
    } else {
        handlers::handle_focus(cmd, &active_client, &ws_info, is_at_edge, &plugins)
    }
}
