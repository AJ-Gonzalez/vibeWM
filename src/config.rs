/// vibeWM configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Gap between windows and screen edges (pixels)
    pub outer_gap: i32,

    /// Gap between snapped windows (pixels)
    pub inner_gap: i32,

    /// Window move step size (pixels)
    pub move_step: i32,

    /// Window resize step size (pixels)
    pub resize_step: i32,

    /// Border width (pixels)
    pub border_width: i32,

    /// Colors - vibecode af
    pub colors: Colors,
}

#[derive(Debug, Clone)]
pub struct Colors {
    /// Background color
    pub background: [f32; 4],

    /// Focused window border
    pub border_focused: [f32; 4],

    /// Unfocused window border
    pub border_unfocused: [f32; 4],

    /// Command center background
    pub command_center_bg: [f32; 4],

    /// Accent color
    pub accent: [f32; 4],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Modifier key is always Super/Logo - checked via modifiers.logo in input.rs
            outer_gap: 10,
            inner_gap: 10,
            move_step: 50,
            resize_step: 50,
            border_width: 2,
            colors: Colors::default(),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        // Vibecode aesthetic - dark with neon accents
        Self {
            // Deep dark background
            background: [0.05, 0.05, 0.08, 1.0],

            // Neon cyan for focused
            border_focused: [0.0, 0.9, 0.9, 1.0],

            // Dim gray for unfocused
            border_unfocused: [0.3, 0.3, 0.35, 1.0],

            // Slightly lighter dark for command center
            command_center_bg: [0.08, 0.08, 0.12, 0.95],

            // Hot pink accent
            accent: [1.0, 0.2, 0.6, 1.0],
        }
    }
}

/// Snap positions for windows
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapPosition {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Maximize,
    Center,
}
