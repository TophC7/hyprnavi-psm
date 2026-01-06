<h1>
  <picture>
    <source srcset="https://fonts.gstatic.com/s/e/notoemoji/latest/1f4a7/512.webp" type="image/webp">
    <img src="https://fonts.gstatic.com/s/e/notoemoji/latest/1f4a7/512.gif" alt="💧" width="32" height="32">
  </picture>
  hyprnavi
</h1>
Smart navigation for Hyprland. Move focus between windows, and seamlessly continue to adjacent workspaces or monitors at screen edges.

Built for [hyprscrolling](https://github.com/hyprwm/hyprland-plugins/tree/main/hyprscrolling) and [split-monitor-workspaces](https://github.com/Duckonaut/split-monitor-workspaces), but works great standalone.

## Installation

### Nix

```bash
nix build github:tophc7/hyprnavi-psm
# or
nix profile install github:tophc7/hyprnavi-psm
```

### From source

```bash
git clone https://github.com/tophc7/hyprnavi-psm.git
cd hyprnavi-psm
cargo build --release
cp target/release/hyprnavi ~/.local/bin/
```

## Usage

```
hyprnavi <direction> [options]

Directions:
  r, l, u, d    Right, Left, Up, Down

Options:
  -s, --swap       Move/swap window instead of focus
  -m, --monitor    At edge, go to monitor instead of workspace
  -p, --position   Position-based edge detection (for scrolling layouts)
  -n, --no-wrap    Don't wrap from last to first workspace
  -b, --bordersize Border tolerance for edge detection (default: 0)
```

## Example Config

```conf
# Focus navigation
bind = SUPER, H, exec, hyprnavi l
bind = SUPER, L, exec, hyprnavi r
bind = SUPER, K, exec, hyprnavi u
bind = SUPER, J, exec, hyprnavi d

# Move windows
bind = SUPER SHIFT, H, exec, hyprnavi l -s
bind = SUPER SHIFT, L, exec, hyprnavi r -s
bind = SUPER SHIFT, K, exec, hyprnavi u -s
bind = SUPER SHIFT, J, exec, hyprnavi d -s
```

### With hyprscrolling

hyprscrolling arranges windows in columns (vertical stacks). Use `-p` for position-based edge detection that works with off-screen windows:

```conf
# Focus with scrolling support
bind = SUPER, H, exec, hyprnavi l -p
bind = SUPER, L, exec, hyprnavi r -p
bind = SUPER, K, exec, hyprnavi u -p
bind = SUPER, J, exec, hyprnavi d -p

# Move windows (column-aware)
# - l/r: promotes window to its own column, then moves to monitor
# - u/d: moves within column, then to adjacent workspace
bind = SUPER SHIFT, H, exec, hyprnavi l -psm
bind = SUPER SHIFT, L, exec, hyprnavi r -psm
bind = SUPER SHIFT, K, exec, hyprnavi u -ps
bind = SUPER SHIFT, J, exec, hyprnavi d -ps
```

### With split-monitor-workspaces

The plugin is auto-detected. When active, hyprnavi uses `split-workspace` for proper per-monitor workspace navigation.

## Behavior Summary

| Flags   | At Edge            | Not at Edge            |
| ------- | ------------------ | ---------------------- |
| (none)  | Next workspace     | Focus neighbor         |
| `-m`    | Focus monitor      | Focus neighbor         |
| `-p`    | Next workspace     | Focus neighbor*        |
| `-pm`   | Focus monitor      | Focus neighbor*        |
| `-s`    | Move to workspace  | Swap with neighbor     |
| `-sm`   | Move to monitor    | Swap with neighbor     |
| `-ps`   | Move to workspace  | Promote in column*     |
| `-psm`  | Move to monitor**  | Promote in column*     |

*With hyprscrolling: uses `layoutmsg focus/movewindowto` for proper scrolling behavior.

**Only moves to monitor when window is already alone in its column (can't promote further). This prioritizes column promotion over monitor movement.

Add `-n` to disable workspace wrapping.

## License

MIT
