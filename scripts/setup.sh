#!/bin/bash
#
# vibeWM Setup Script
# Sets up everything needed to build vibeWM on Ubuntu/Debian
#

set -e

echo "================================================"
echo "  vibeWM Setup"
echo "  The anti-suckless Wayland compositor"
echo "================================================"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Check if running as root (we don't want that)
if [ "$EUID" -eq 0 ]; then
    echo -e "${RED}Don't run this script as root!${NC}"
    echo "Run as your normal user - it will use sudo when needed."
    exit 1
fi

# Detect distro
if [ -f /etc/debian_version ]; then
    DISTRO="debian"
elif [ -f /etc/fedora-release ]; then
    DISTRO="fedora"
elif [ -f /etc/arch-release ]; then
    DISTRO="arch"
else
    DISTRO="unknown"
fi

echo -e "${CYAN}Detected distro:${NC} $DISTRO"
echo

# Install system dependencies
echo -e "${CYAN}Installing system dependencies...${NC}"

case $DISTRO in
    debian)
        sudo apt update
        sudo apt install -y \
            build-essential \
            pkg-config \
            cmake \
            libwayland-dev \
            libxkbcommon-dev \
            libudev-dev \
            libinput-dev \
            libgbm-dev \
            libdrm-dev \
            libegl-dev \
            libgles2-mesa-dev \
            libseat-dev \
            curl
        ;;
    fedora)
        sudo dnf install -y \
            gcc \
            pkg-config \
            cmake \
            wayland-devel \
            libxkbcommon-devel \
            systemd-devel \
            libinput-devel \
            mesa-libgbm-devel \
            libdrm-devel \
            mesa-libEGL-devel \
            mesa-libGLES-devel \
            libseat-devel \
            curl
        ;;
    arch)
        sudo pacman -S --needed \
            base-devel \
            pkg-config \
            cmake \
            wayland \
            libxkbcommon \
            systemd \
            libinput \
            mesa \
            libdrm \
            seatd \
            curl
        ;;
    *)
        echo -e "${RED}Unknown distro. Please install dependencies manually:${NC}"
        echo "  - build tools (gcc, make, pkg-config, cmake)"
        echo "  - libwayland-dev"
        echo "  - libxkbcommon-dev"
        echo "  - libudev-dev"
        echo "  - libinput-dev"
        echo "  - libgbm-dev"
        echo "  - libdrm-dev"
        echo "  - libegl-dev"
        echo "  - libgles2-mesa-dev"
        echo "  - libseat-dev"
        exit 1
        ;;
esac

echo -e "${GREEN}System dependencies installed!${NC}"
echo

# Install Rust if not present
if command -v rustc &> /dev/null; then
    echo -e "${GREEN}Rust already installed:${NC} $(rustc --version)"
else
    echo -e "${CYAN}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}Rust installed:${NC} $(rustc --version)"
fi
echo

# Set up XDG_RUNTIME_DIR if not set
if [ -z "$XDG_RUNTIME_DIR" ]; then
    echo -e "${CYAN}Setting up XDG_RUNTIME_DIR...${NC}"
    export XDG_RUNTIME_DIR="/tmp/runtime-$USER"
    mkdir -p "$XDG_RUNTIME_DIR"

    # Add to bashrc if not already there
    if ! grep -q "XDG_RUNTIME_DIR" ~/.bashrc; then
        echo "" >> ~/.bashrc
        echo "# vibeWM - Wayland runtime dir" >> ~/.bashrc
        echo 'export XDG_RUNTIME_DIR="/tmp/runtime-$USER"' >> ~/.bashrc
        echo 'mkdir -p "$XDG_RUNTIME_DIR" 2>/dev/null' >> ~/.bashrc
        echo -e "${GREEN}Added XDG_RUNTIME_DIR to ~/.bashrc${NC}"
    fi
fi
echo

# Build vibeWM
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${CYAN}Building vibeWM...${NC}"
cd "$PROJECT_DIR"

# Source cargo env in case it was just installed
source "$HOME/.cargo/env" 2>/dev/null || true

# Build both backends
echo "Building windowed backend..."
cargo build --release

echo "Building bare metal backend..."
cargo build --release --features udev

echo
echo -e "${GREEN}================================================${NC}"
echo -e "${GREEN}  vibeWM is ready!${NC}"
echo -e "${GREEN}================================================${NC}"
echo
echo "Binaries built at:"
echo "  ./target/release/vibewm"
echo
echo "To run (windowed, inside a DE):"
echo "  ./target/release/vibewm"
echo
echo "To run (bare metal, from TTY):"
echo "  1. Press Ctrl+Alt+F2 to switch to TTY"
echo "  2. Log in"
echo "  3. Run: ./target/release/vibewm"
echo "  4. Press Ctrl+Alt+F1 to get back"
echo
echo -e "${CYAN}Enjoy the vibes!${NC}"
