# hyprnavi-psm

Smart navigation tool for Hyprland with edge-detection. Navigate seamlessly between windows, workspaces, and monitors using a unified interface for all four directions.

## Features

- **Unified navigation**: All 4 directions (up/down/left/right) with consistent behavior
- **Smart edge detection**: Two modes available:
  - **Pixel mode** (default): Detects when window is at monitor boundary
  - **Position mode** (`-p`): Detects when window is the extreme (leftmost/rightmost/etc.)
- **Configurable edge behavior**:
  - Default: Switch to adjacent **workspace** when at edge
  - With `-m` flag: Switch to adjacent **monitor** when at edge
- **Swap mode** (`-s`): Move windows instead of just changing focus
- **Swap + Monitor** (`-s -m`): At edge, explicitly move window to adjacent monitor
- **Floating window support**: Properly handles floating windows

## Installation

### Nix (recommended)

```bash
# Enter development shell
nix develop

# Build
cargo build --release

# Or build with Nix directly
nix build
```

### Manual

```bash
git clone https://github.com/tophc7/hyprnavi-psm.git
cd hyprnavi-psm
cargo build --release
sudo cp target/release/hyprnavi /usr/local/bin
```

## Usage

```
hyprnavi <command> [options]

Commands:
  r    Focus right. At edge: next workspace (default) or right monitor (-m)
  l    Focus left. At edge: previous workspace (default) or left monitor (-m)
  u    Focus up. At edge: previous workspace (default) or upper monitor (-m)
  d    Focus down. At edge: next workspace (default) or lower monitor (-m)

Options:
  -s, --swap         Swap/move window instead of changing focus
  -m, --monitor      At edge, switch to adjacent monitor instead of workspace
  -p, --position     Use position-based edge detection (am I the extreme window?)
  -b, --bordersize   Window border size tolerance for pixel mode (default: 0)
  -V, --version      Print version information
  --help             Display usage information
```

## Edge Detection Modes

### Pixel Mode (default)
Checks if the window is physically at the monitor's edge:
```
window.left_edge near monitor.left_edge?
```
Best for: Traditional tiled layouts where windows fill the screen.

### Position Mode (`-p`)
Checks if this is the extreme window in that direction:
```
Am I the leftmost/rightmost/topmost/bottommost window?
```
Best for: Scrolling layouts (hyprscroller) where windows can be off-screen.

## Example Configuration

### Hyprscroller (vertical workspaces, position mode)

For hyprscroller with vertical workspace layouts:

```conf
# Workspaces are up/down
bind = SUPER, K, exec, hyprnavi u
bind = SUPER, J, exec, hyprnavi d
bind = SUPER SHIFT, K, exec, hyprnavi u -s
bind = SUPER SHIFT, J, exec, hyprnavi d -s

# Monitors are left/right (use position mode for proper edge detection)
bind = SUPER, H, exec, hyprnavi l -p -m
bind = SUPER, L, exec, hyprnavi r -p -m
bind = SUPER SHIFT, H, exec, hyprnavi l -p -s -m
bind = SUPER SHIFT, L, exec, hyprnavi r -p -s -m
```

## Behavior Summary

| Flags | At Edge | Not at Edge |
|-------|---------|-------------|
| (none) | Switch to adjacent workspace | Focus neighbor window |
| `-m` | Switch to adjacent monitor | Focus neighbor window |
| `-s` | Move window (Hyprland default) | Swap with neighbor |
| `-s -m` | Move window to adjacent monitor | Swap with neighbor |
| `-p` | Uses position-based detection | Uses position-based detection |

### Edge Detection Comparison

| Mode | Check | Use Case |
|------|-------|----------|
| Pixel (default) | Is window at screen boundary? | Traditional tiled layouts |
| Position (`-p`) | Is this the extreme window? | Scrolling layouts (hyprscroller) |

## License

MIT
