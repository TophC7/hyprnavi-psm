use argh::FromArgs;

/// simple horizontal navigation in hyprland
#[derive(FromArgs)]
pub struct Flags {
    #[argh(subcommand)]
    pub cmd: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum Command {
    Right(CommandRight),
    Left(CommandLeft),
    Up(CommandUp),
    Down(CommandDown),
}

/// Focus on the next window. If the current window is already at the edge, focus on the next workspace.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "r")]
pub struct CommandRight {
    #[argh(switch, description = "swap window")]
    pub swap: bool,
    #[argh(
        option,
        description = "window border size. Necessary for boundary detection"
    )]
    pub bordersize: Option<i32>,
}

/// Focus on the previous window. If the current window is already at the edge, focus on the previous workspace.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "l")]
pub struct CommandLeft {
    #[argh(switch, description = "swap window")]
    pub swap: bool,
    #[argh(
        option,
        description = "window border size. Necessary for boundary detection"
    )]
    pub bordersize: Option<i32>,
}

/// Focus on the next window. If the current window is already at the edge, focus on the next workspace.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "u")]
pub struct CommandUp {
    #[argh(switch, description = "swap window")]
    pub swap: bool,
}

/// Focus on the next window. If the current window is already at the edge, focus on the next workspace.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "d")]
pub struct CommandDown {
    #[argh(switch, description = "swap window")]
    pub swap: bool,
}
