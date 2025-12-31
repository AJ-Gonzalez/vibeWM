# vibeWM

> *The anti-suckless, spite-driven, vibecoded Wayland compositor*

No minimalism. No plaintext configs. No 200-line header file flexing. Just **vibes**.

```
    ██╗   ██╗██╗██████╗ ███████╗██╗    ██╗███╗   ███╗
    ██║   ██║██║██╔══██╗██╔════╝██║    ██║████╗ ████║
    ██║   ██║██║██████╔╝█████╗  ██║ █╗ ██║██╔████╔██║
    ╚██╗ ██╔╝██║██╔══██╗██╔══╝  ██║███╗██║██║╚██╔╝██║
     ╚████╔╝ ██║██████╔╝███████╗╚███╔███╔╝██║ ╚═╝ ██║
      ╚═══╝  ╚═╝╚═════╝ ╚══════╝ ╚══╝╚══╝ ╚═╝     ╚═╝
```

## Philosophy

- **Stacking, not tiling** - windows go where you want them
- **Vim motions** - because muscle memory is real
- **Command Center** - no status bar, just a beautiful overlay
- **Aesthetic maximalism** - glows, gradients, glass, the works

## Keybinds

| Key | Action |
|-----|--------|
| `mod+i` | Move window up |
| `mod+k` | Move window down |
| `mod+j` | Move window left |
| `mod+l` | Move window right |
| `mod+R` + `ijkl` | Resize window (hold R) |
| `mod+←` | Snap to left half |
| `mod+→` | Snap to right half |
| `mod+↑` | Snap to top half |
| `mod+↓` | Snap to bottom half |
| `mod+S` | **Command Center** |
| `mod+Tab` | Cycle focus |
| `mod+W` | Close window |
| `mod+Q` | Quit |

### Command Center

Press `mod+S` and experience:
- **Fuzzy app launcher** - just start typing
- **Window list** as clickable tiles
- **Clock, battery, system info** - all the panel stuff, but pretty
- **Smooth animations** - staggered entrance, glow pulse, glass blur
- **Arrow keys** to navigate, **Enter** to launch, **Escape** to close

## The Aesthetic

```
Background:     Deep space black (#0d0d14)
Focused:        Neon cyan that HITS (#00e6e6)
Accent:         Hot pink that SLAPS (#ff3399)
Tertiary:       Electric purple (#9933ff)
```

- Glowing borders that **breathe**
- Gradient overlays
- Glass blur on Command Center
- Staggered animations because we have taste
- Rounded corners because it's not 1995

## Quick Start (Linux VM or Bare Metal)

```bash
# One-liner setup (Ubuntu/Debian)
./scripts/setup.sh

# Build and run
cargo build --release
./target/release/vibewm
```

## Building

### Two Backends

| Backend | Use Case | Build Command |
|---------|----------|---------------|
| **Winit** | Dev/testing in a window (inside a DE) | `cargo build --release` |
| **DRM** | Bare metal, owns the display (no DE) | `cargo build --release --features udev` |

### Full Setup (Ubuntu/Debian)

```bash
# Install dependencies
sudo apt update && sudo apt install -y \
    build-essential pkg-config cmake \
    libwayland-dev libxkbcommon-dev libudev-dev \
    libinput-dev libgbm-dev libdrm-dev libegl-dev \
    libgles2-mesa-dev libseat-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Build (windowed mode)
cargo build --release

# Build (bare metal mode)
cargo build --release --features udev
```

### Running

```bash
# Required env var
export XDG_RUNTIME_DIR=/tmp/runtime-$USER
mkdir -p $XDG_RUNTIME_DIR

# Windowed mode (inside GNOME/KDE/etc)
./target/release/vibewm

# Bare metal mode (from TTY - Ctrl+Alt+F2)
./target/release/vibewm  # (built with --features udev)
```

### VirtualBox VM Setup

If testing in a VM:
- **RAM**: 4GB minimum
- **Video Memory**: 128MB+
- **Graphics Controller**: VBoxSVGA or VMSVGA
- **Enable**: 3D Acceleration

### WSL (Limited)

WSL can compile but WSLg may not display the window (depends on your setup):

```bash
wsl --install -d Ubuntu
# Then run setup.sh inside WSL
```

## Tech

- **Rust** - no segfaults at 4am
- **Smithay** - Wayland compositor library
- **Custom shaders** - glow, gradients, glass effects

## Why?

Because [suckless](https://suckless.org/) is great but sometimes you want a window manager that doesn't look like it runs on a VT100. This is for people who:

- Want their desktop to feel *alive*
- Think eye candy is a feature, not bloat
- Use vim keybindings but also appreciate aesthetics
- Are tired of pretending they don't care how things look

## License

MIT - do whatever you want

---

*Built with spite and good taste by AJ Gonzalez*
