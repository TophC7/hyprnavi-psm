use clap::{Parser, Subcommand};
use hyprland::dispatch::Direction;

/// Simple navigation tool for Hyprland with smart edge-detection.
///
/// At screen edges, can switch to adjacent workspace (default) or monitor (-m flag).
#[derive(Parser)]
#[command(name = "hyprnavi", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Focus right. At edge: next workspace (default) or right monitor (-m)
    #[command(name = "r")]
    Right(NavArgs),
    /// Focus left. At edge: previous workspace (default) or left monitor (-m)
    #[command(name = "l")]
    Left(NavArgs),
    /// Focus up. At edge: previous workspace (default) or upper monitor (-m)
    #[command(name = "u")]
    Up(NavArgs),
    /// Focus down. At edge: next workspace (default) or lower monitor (-m)
    #[command(name = "d")]
    Down(NavArgs),
}

impl Command {
    pub fn direction(&self) -> Direction {
        match self {
            Self::Up(_) => Direction::Up,
            Self::Down(_) => Direction::Down,
            Self::Left(_) => Direction::Left,
            Self::Right(_) => Direction::Right,
        }
    }

    pub fn args(&self) -> &NavArgs {
        match self {
            Self::Up(a) | Self::Down(a) | Self::Left(a) | Self::Right(a) => a,
        }
    }

    /// Right/down are "forward" (toward next workspace).
    pub fn is_forward(&self) -> bool {
        matches!(self, Self::Right(_) | Self::Down(_))
    }
}

#[derive(Parser, Clone, Debug)]
pub struct NavArgs {
    /// Swap window instead of moving focus
    #[arg(short, long)]
    pub swap: bool,

    /// Move to adjacent monitor instead of workspace when at edge
    #[arg(short, long)]
    pub monitor: bool,

    /// Use position-based edge detection (is this the extreme window?)
    #[arg(short, long)]
    pub position: bool,

    /// Disable workspace wrapping (don't go from last to first)
    /// Note: When split-monitor-workspaces plugin is active, wrapping is
    /// controlled by the plugin's `enable_wrapping` config instead.
    #[arg(short = 'n', long = "no-wrap")]
    pub no_wrap: bool,

    /// Window border size for boundary detection tolerance (pixel mode only)
    #[arg(short, long, default_value_t = 0)]
    pub bordersize: i32,
}
