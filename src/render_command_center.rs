//! Command Center Rendering - Where the vibes become pixels
//!
//! This is the anti-suckless manifesto in code form.
//! Every pixel drips with intention.

use crate::command_center::{CommandCenter, CommandCenterLayout, CommandCenterTheme};

/// Render data for a single frame
pub struct CommandCenterFrame {
    /// Background quad with blur
    pub background: RenderQuad,

    /// Gradient overlay
    pub gradient: GradientQuad,

    /// Glow border around container
    pub glow: GlowEffect,

    /// Search bar
    pub search_bar: SearchBarRender,

    /// App cards
    pub app_cards: Vec<AppCardRender>,

    /// System info bar
    pub system_bar: SystemBarRender,

    /// Overall opacity (for open/close animation)
    pub opacity: f32,

    /// Scale (for juicy open animation)
    pub scale: f32,
}

#[derive(Clone)]
pub struct RenderQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
    pub corner_radius: f32,
}

pub struct GradientQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub angle: f32,  // radians
}

pub struct GlowEffect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 4],
    pub intensity: f32,
    pub spread: f32,
    pub corner_radius: f32,
}

pub struct SearchBarRender {
    pub background: RenderQuad,
    pub text: TextRender,
    pub cursor: CursorRender,
    pub icon: IconRender,
}

pub struct TextRender {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: [f32; 4],
    pub size: f32,
    pub font_weight: FontWeight,
}

#[derive(Clone, Copy)]
pub enum FontWeight {
    Regular,
    Medium,
    Bold,
}

pub struct CursorRender {
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub color: [f32; 4],
    pub blink_phase: f32,  // For blinking animation
}

pub struct IconRender {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub icon: Icon,
    pub color: [f32; 4],
}

#[derive(Clone, Copy)]
pub enum Icon {
    Search,
    Battery(u8, bool),  // percent, charging
    Clock,
    Cpu,
    Memory,
    App,
    Window,
    Close,
}

pub struct AppCardRender {
    pub background: RenderQuad,
    pub icon: Option<IconRender>,
    pub name: TextRender,
    pub selected: bool,
    pub hover_t: f32,  // Animation progress
    pub stagger_delay: f32,  // For staggered entrance
}

pub struct SystemBarRender {
    pub background: RenderQuad,
    pub clock: TextRender,
    pub battery: BatteryRender,
    pub dividers: Vec<RenderQuad>,
}

pub struct BatteryRender {
    pub icon: IconRender,
    pub text: TextRender,
    pub bar_background: RenderQuad,
    pub bar_fill: RenderQuad,
}

impl CommandCenter {
    /// Generate render data for current frame
    pub fn render(&self, layout: &CommandCenterLayout, theme: &CommandCenterTheme) -> CommandCenterFrame {
        let t = self.animation_t;

        // Easing function - cubic ease out for that smooth feeling
        let eased_t = 1.0 - (1.0 - t).powi(3);

        // Scale animation - starts slightly small, grows to full size
        let scale = 0.95 + 0.05 * eased_t;

        // Container dimensions with scale applied
        let container_x = layout.container_x as f32;
        let container_y = layout.container_y as f32;
        let container_w = layout.container_width as f32;
        let container_h = layout.container_height as f32;

        // Center point for scaling
        let center_x = container_x + container_w / 2.0;
        let center_y = container_y + container_h / 2.0;

        // Scaled container
        let scaled_w = container_w * scale;
        let scaled_h = container_h * scale;
        let scaled_x = center_x - scaled_w / 2.0;
        let scaled_y = center_y - scaled_h / 2.0;

        CommandCenterFrame {
            background: RenderQuad {
                x: scaled_x,
                y: scaled_y,
                width: scaled_w,
                height: scaled_h,
                color: theme.bg_color,
                corner_radius: 16.0,
            },

            gradient: GradientQuad {
                x: scaled_x,
                y: scaled_y,
                width: scaled_w,
                height: scaled_h,
                color_start: theme.gradient_start,
                color_end: theme.gradient_end,
                angle: 45.0_f32.to_radians(),
            },

            glow: GlowEffect {
                x: scaled_x,
                y: scaled_y,
                width: scaled_w,
                height: scaled_h,
                color: with_alpha(theme.glow_color, self.current_glow() * eased_t),
                intensity: theme.glow_intensity,
                spread: 20.0,
                corner_radius: 16.0,
            },

            search_bar: self.render_search_bar(layout, theme, eased_t),
            app_cards: self.render_app_cards(layout, theme, eased_t),
            system_bar: self.render_system_bar(layout, theme, eased_t),

            opacity: eased_t,
            scale,
        }
    }

    fn render_search_bar(&self, layout: &CommandCenterLayout, theme: &CommandCenterTheme, t: f32) -> SearchBarRender {
        let x = layout.search_x as f32;
        let y = layout.search_y as f32;
        let w = layout.search_width as f32;
        let h = layout.search_height as f32;

        // Stagger: search bar comes in first
        let local_t = ((t - 0.0) * 2.0).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - local_t).powi(3);

        let offset_y = 20.0 * (1.0 - eased);

        SearchBarRender {
            background: RenderQuad {
                x,
                y: y + offset_y,
                width: w,
                height: h,
                color: theme.card_bg,
                corner_radius: theme.card_border_radius,
            },
            icon: IconRender {
                x: x + 16.0,
                y: y + offset_y + h / 2.0,
                size: 20.0,
                icon: Icon::Search,
                color: theme.text_secondary,
            },
            text: TextRender {
                x: x + 48.0,
                y: y + offset_y + h / 2.0,
                text: if self.search_query.is_empty() {
                    "Search apps...".to_string()
                } else {
                    self.search_query.clone()
                },
                color: if self.search_query.is_empty() {
                    theme.text_secondary
                } else {
                    theme.text_primary
                },
                size: 18.0,
                font_weight: FontWeight::Regular,
            },
            cursor: CursorRender {
                x: x + 48.0 + self.search_query.len() as f32 * 10.0, // Approximate
                y: y + offset_y + 12.0,
                height: h - 24.0,
                color: theme.accent_primary,
                blink_phase: self.glow_phase,  // Reuse for cursor blink
            },
        }
    }

    fn render_app_cards(&self, layout: &CommandCenterLayout, theme: &CommandCenterTheme, t: f32) -> Vec<AppCardRender> {
        let start_x = layout.apps_x as f32;
        let start_y = layout.apps_y as f32;
        let card_w = layout.app_card_width as f32;
        let card_h = layout.app_card_height as f32;
        let columns = layout.app_columns as usize;
        let gap = 12.0;

        self.filtered_apps
            .iter()
            .take(12)  // Max visible
            .enumerate()
            .map(|(i, app)| {
                let col = i % columns;
                let row = i / columns;

                let x = start_x + col as f32 * (card_w + gap);
                let y = start_y + row as f32 * (card_h + gap);

                // Stagger animation - each card delayed slightly
                let delay = 0.1 + i as f32 * 0.03;
                let local_t = ((t - delay) * 3.0).clamp(0.0, 1.0);
                let eased = 1.0 - (1.0 - local_t).powi(3);

                let offset_y = 30.0 * (1.0 - eased);
                let card_opacity = eased;

                let selected = i == self.selected_index;

                AppCardRender {
                    background: RenderQuad {
                        x,
                        y: y + offset_y,
                        width: card_w,
                        height: card_h,
                        color: if selected {
                            with_alpha(theme.card_selected, card_opacity)
                        } else {
                            with_alpha(theme.card_bg, card_opacity)
                        },
                        corner_radius: theme.card_border_radius,
                    },
                    icon: Some(IconRender {
                        x: x + 16.0,
                        y: y + offset_y + card_h / 2.0,
                        size: 24.0,
                        icon: Icon::App,
                        color: with_alpha(
                            if selected { theme.accent_primary } else { theme.text_secondary },
                            card_opacity
                        ),
                    }),
                    name: TextRender {
                        x: x + 52.0,
                        y: y + offset_y + card_h / 2.0,
                        text: truncate_string(&app.name, 15),
                        color: with_alpha(
                            if selected { theme.text_highlight } else { theme.text_primary },
                            card_opacity
                        ),
                        size: 14.0,
                        font_weight: if selected { FontWeight::Medium } else { FontWeight::Regular },
                    },
                    selected,
                    hover_t: 0.0,
                    stagger_delay: delay,
                }
            })
            .collect()
    }

    fn render_system_bar(&self, layout: &CommandCenterLayout, theme: &CommandCenterTheme, t: f32) -> SystemBarRender {
        let x = layout.system_x as f32;
        let y = layout.system_y as f32;
        let w = layout.system_width as f32;
        let h = layout.system_height as f32;

        // System bar comes in last
        let delay = 0.3;
        let local_t = ((t - delay) * 2.0).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - local_t).powi(3);

        let offset_y = 20.0 * (1.0 - eased);

        let sys_info = self.get_system_info();

        SystemBarRender {
            background: RenderQuad {
                x,
                y: y + offset_y,
                width: w,
                height: h,
                color: with_alpha(theme.card_bg, eased * 0.5),
                corner_radius: theme.card_border_radius,
            },
            clock: TextRender {
                x: x + 16.0,
                y: y + offset_y + h / 2.0,
                text: self.get_time_string(),
                color: with_alpha(theme.text_primary, eased),
                size: 16.0,
                font_weight: FontWeight::Medium,
            },
            battery: BatteryRender {
                icon: IconRender {
                    x: x + w - 100.0,
                    y: y + offset_y + h / 2.0,
                    size: 18.0,
                    icon: Icon::Battery(sys_info.battery_percent, sys_info.battery_charging),
                    color: with_alpha(
                        if sys_info.battery_percent < 20 {
                            theme.accent_secondary  // Warning color
                        } else {
                            theme.text_secondary
                        },
                        eased
                    ),
                },
                text: TextRender {
                    x: x + w - 75.0,
                    y: y + offset_y + h / 2.0,
                    text: format!("{}%", sys_info.battery_percent),
                    color: with_alpha(theme.text_secondary, eased),
                    size: 14.0,
                    font_weight: FontWeight::Regular,
                },
                bar_background: RenderQuad {
                    x: x + w - 45.0,
                    y: y + offset_y + h / 2.0 - 6.0,
                    width: 30.0,
                    height: 12.0,
                    color: with_alpha([0.3, 0.3, 0.3, 1.0], eased),
                    corner_radius: 3.0,
                },
                bar_fill: RenderQuad {
                    x: x + w - 44.0,
                    y: y + offset_y + h / 2.0 - 5.0,
                    width: 28.0 * (sys_info.battery_percent as f32 / 100.0),
                    height: 10.0,
                    color: with_alpha(
                        if sys_info.battery_charging {
                            theme.accent_primary
                        } else if sys_info.battery_percent < 20 {
                            theme.accent_secondary
                        } else {
                            [0.3, 0.9, 0.4, 1.0]  // Green
                        },
                        eased
                    ),
                    corner_radius: 2.0,
                },
            },
            dividers: vec![
                // Vertical divider between clock and battery
                RenderQuad {
                    x: x + w - 120.0,
                    y: y + offset_y + 8.0,
                    width: 1.0,
                    height: h - 16.0,
                    color: with_alpha([1.0, 1.0, 1.0, 0.1], eased),
                    corner_radius: 0.0,
                }
            ],
        }
    }
}

// Helper functions

fn with_alpha(color: [f32; 4], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], color[3] * alpha]
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// GLSL shader source for the glow effect
/// This is the good stuff - the actual GPU magic
pub const GLOW_SHADER_FRAG: &str = r#"
#version 300 es
precision highp float;

uniform vec4 u_color;
uniform vec2 u_size;
uniform float u_radius;
uniform float u_spread;
uniform float u_intensity;

in vec2 v_uv;
out vec4 frag_color;

float rounded_box_sdf(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    vec2 p = (v_uv - 0.5) * u_size;
    vec2 b = u_size * 0.5;

    float d = rounded_box_sdf(p, b, u_radius);

    // Soft glow falloff
    float glow = 1.0 - smoothstep(0.0, u_spread, d);
    glow = pow(glow, 1.5) * u_intensity;

    frag_color = vec4(u_color.rgb, u_color.a * glow);
}
"#;

/// GLSL shader for gradient backgrounds
pub const GRADIENT_SHADER_FRAG: &str = r#"
#version 300 es
precision highp float;

uniform vec4 u_color_start;
uniform vec4 u_color_end;
uniform float u_angle;

in vec2 v_uv;
out vec4 frag_color;

void main() {
    // Rotate UV for angled gradient
    float s = sin(u_angle);
    float c = cos(u_angle);
    vec2 rotated = vec2(
        v_uv.x * c - v_uv.y * s,
        v_uv.x * s + v_uv.y * c
    );

    float t = rotated.x + 0.5;
    t = clamp(t, 0.0, 1.0);

    frag_color = mix(u_color_start, u_color_end, t);
}
"#;

/// GLSL shader for glass/blur effect (simplified - real blur needs multiple passes)
pub const GLASS_SHADER_FRAG: &str = r#"
#version 300 es
precision highp float;

uniform sampler2D u_background;
uniform vec4 u_tint;
uniform vec2 u_size;
uniform float u_radius;
uniform float u_blur;

in vec2 v_uv;
out vec4 frag_color;

float rounded_box_sdf(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    vec2 p = (v_uv - 0.5) * u_size;
    vec2 b = u_size * 0.5;

    float d = rounded_box_sdf(p, b, u_radius);

    if (d > 0.0) {
        discard;
    }

    // Simple box blur (for real blur, use Kawase or Gaussian with multiple passes)
    vec4 color = vec4(0.0);
    float total = 0.0;

    for (float x = -2.0; x <= 2.0; x += 1.0) {
        for (float y = -2.0; y <= 2.0; y += 1.0) {
            vec2 offset = vec2(x, y) * u_blur / u_size;
            color += texture(u_background, v_uv + offset);
            total += 1.0;
        }
    }

    color /= total;

    // Apply tint
    frag_color = mix(color, u_tint, u_tint.a);
}
"#;
