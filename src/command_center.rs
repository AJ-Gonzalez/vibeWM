//! Command Center - The anti-suckless control panel
//!
//! No status bars. No minimalism. Just vibes.
//! Press mod+S and bask in the glow.

use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;

/// The Command Center state
pub struct CommandCenter {
    /// Is visible?
    pub visible: bool,

    /// Animation progress (0.0 = closed, 1.0 = fully open)
    pub animation_t: f32,

    /// When animation started
    pub animation_start: Option<Instant>,

    /// Current search query
    pub search_query: String,

    /// Filtered app list
    pub filtered_apps: Vec<AppEntry>,

    /// All available apps
    pub all_apps: Vec<AppEntry>,

    /// Selected index in the list
    pub selected_index: usize,

    /// Current section focus
    pub section: CommandCenterSection,

    /// Glow pulse phase (for that sweet sweet animation)
    pub glow_phase: f32,

    /// Last frame time for animations
    pub last_frame: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandCenterSection {
    Search,
    Apps,
    Windows,
    System,
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub desktop_file: PathBuf,
    /// Fuzzy match score (higher = better match)
    pub score: i32,
}

/// Visual theme - DRIPPING with vibes
pub struct CommandCenterTheme {
    // Background
    pub bg_color: [f32; 4],
    pub bg_blur_radius: f32,

    // The iconic gradient
    pub gradient_start: [f32; 4],
    pub gradient_end: [f32; 4],

    // Glow effects
    pub glow_color: [f32; 4],
    pub glow_intensity: f32,
    pub glow_pulse_speed: f32,

    // Accent colors
    pub accent_primary: [f32; 4],    // Neon cyan
    pub accent_secondary: [f32; 4],  // Hot pink
    pub accent_tertiary: [f32; 4],   // Electric purple

    // Text
    pub text_primary: [f32; 4],
    pub text_secondary: [f32; 4],
    pub text_highlight: [f32; 4],

    // Cards/tiles
    pub card_bg: [f32; 4],
    pub card_hover: [f32; 4],
    pub card_selected: [f32; 4],
    pub card_border_radius: f32,

    // Animation
    pub open_duration_ms: f32,
    pub hover_transition_ms: f32,
    pub stagger_delay_ms: f32,
}

impl Default for CommandCenterTheme {
    fn default() -> Self {
        Self {
            // Deep space background with transparency
            bg_color: [0.02, 0.02, 0.05, 0.92],
            bg_blur_radius: 20.0,

            // Sunset/synthwave gradient
            gradient_start: [0.4, 0.0, 0.6, 0.3],   // Purple
            gradient_end: [0.0, 0.8, 0.8, 0.2],     // Cyan

            // Glow that BREATHES
            glow_color: [0.0, 1.0, 1.0, 0.4],
            glow_intensity: 1.2,
            glow_pulse_speed: 2.0,  // Seconds per cycle

            // THE VIBES
            accent_primary: [0.0, 1.0, 0.9, 1.0],    // Cyan that HITS
            accent_secondary: [1.0, 0.2, 0.6, 1.0],  // Pink that SLAPS
            accent_tertiary: [0.6, 0.2, 1.0, 1.0],   // Purple that GOES

            // Crisp text
            text_primary: [1.0, 1.0, 1.0, 1.0],
            text_secondary: [0.7, 0.7, 0.8, 1.0],
            text_highlight: [0.0, 1.0, 0.9, 1.0],

            // Glass cards
            card_bg: [1.0, 1.0, 1.0, 0.05],
            card_hover: [1.0, 1.0, 1.0, 0.1],
            card_selected: [0.0, 1.0, 0.9, 0.2],
            card_border_radius: 12.0,

            // Smooth animations
            open_duration_ms: 200.0,
            hover_transition_ms: 150.0,
            stagger_delay_ms: 30.0,  // Each item animates slightly after the last
        }
    }
}

impl CommandCenter {
    pub fn new() -> Self {
        let mut center = Self {
            visible: false,
            animation_t: 0.0,
            animation_start: None,
            search_query: String::new(),
            filtered_apps: Vec::new(),
            all_apps: Vec::new(),
            selected_index: 0,
            section: CommandCenterSection::Search,
            glow_phase: 0.0,
            last_frame: Instant::now(),
        };

        // Load apps on creation
        center.load_apps();
        center.filtered_apps = center.all_apps.clone();

        center
    }

    /// Toggle visibility with animation
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.animation_start = Some(Instant::now());

        if self.visible {
            // Reset state when opening
            self.search_query.clear();
            self.filtered_apps = self.all_apps.clone();
            self.selected_index = 0;
            self.section = CommandCenterSection::Search;
        }

        tracing::info!(
            "Command Center: {} ~",
            if self.visible { "opening" } else { "closing" }
        );
    }

    /// Update animations - call every frame
    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        // Update glow pulse
        self.glow_phase += dt * (2.0 * std::f32::consts::PI / 2.0); // 2 second cycle
        if self.glow_phase > 2.0 * std::f32::consts::PI {
            self.glow_phase -= 2.0 * std::f32::consts::PI;
        }

        // Update open/close animation
        if let Some(start) = self.animation_start {
            let elapsed = now.duration_since(start).as_millis() as f32;
            let duration = 200.0; // ms

            if self.visible {
                self.animation_t = (elapsed / duration).min(1.0);
            } else {
                self.animation_t = 1.0 - (elapsed / duration).min(1.0);
            }

            // Animation complete
            if elapsed >= duration {
                self.animation_start = None;
            }
        }
    }

    /// Get current glow intensity (pulses smoothly)
    pub fn current_glow(&self) -> f32 {
        let base = 0.8;
        let pulse = 0.2 * (self.glow_phase.sin() + 1.0) / 2.0;
        base + pulse
    }

    /// Handle text input for search
    pub fn handle_char(&mut self, c: char) {
        if self.section == CommandCenterSection::Search {
            self.search_query.push(c);
            self.update_filter();
        }
    }

    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        if self.section == CommandCenterSection::Search {
            self.search_query.pop();
            self.update_filter();
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index < self.filtered_apps.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Launch selected app
    pub fn launch_selected(&mut self) -> Option<String> {
        if let Some(app) = self.filtered_apps.get(self.selected_index) {
            let exec = app.exec.clone();
            tracing::info!("Launching: {}", app.name);

            // Close command center after launch
            self.toggle();

            Some(exec)
        } else {
            None
        }
    }

    /// Update filtered apps based on search query
    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_apps = self.all_apps.clone();
        } else {
            self.filtered_apps = self.all_apps
                .iter()
                .filter_map(|app| {
                    let score = fuzzy_match(&self.search_query, &app.name);
                    if score > 0 {
                        let mut app = app.clone();
                        app.score = score;
                        Some(app)
                    } else {
                        None
                    }
                })
                .collect();

            // Sort by score descending
            self.filtered_apps.sort_by(|a, b| b.score.cmp(&a.score));
        }

        // Reset selection
        self.selected_index = 0;
    }

    /// Load apps from .desktop files
    fn load_apps(&mut self) {
        let app_dirs = [
            "/usr/share/applications",
            "/usr/local/share/applications",
            "~/.local/share/applications",
        ];

        for dir in &app_dirs {
            let path = if dir.starts_with("~") {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(dir.replace("~", &home))
                } else {
                    continue;
                }
            } else {
                PathBuf::from(dir)
            };

            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                        if let Some(app) = parse_desktop_file(&path) {
                            self.all_apps.push(app);
                        }
                    }
                }
            }
        }

        // Sort alphabetically by default
        self.all_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        tracing::info!("Loaded {} apps", self.all_apps.len());
    }

    /// Get formatted time string
    pub fn get_time_string(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Convert to hours:minutes (simple version)
        let hours = (now % 86400) / 3600;
        let minutes = (now % 3600) / 60;

        format!("{:02}:{:02}", hours, minutes)
    }

    /// Get system info for display
    pub fn get_system_info(&self) -> SystemInfo {
        SystemInfo {
            // These would be populated from actual system calls
            battery_percent: read_battery_percent().unwrap_or(100),
            battery_charging: read_battery_charging().unwrap_or(false),
            cpu_usage: 0.0,  // TODO: implement
            memory_used_gb: 0.0,
            memory_total_gb: 0.0,
        }
    }
}

pub struct SystemInfo {
    pub battery_percent: u8,
    pub battery_charging: bool,
    pub cpu_usage: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
}

/// Fuzzy matching - returns score (0 = no match)
fn fuzzy_match(query: &str, target: &str) -> i32 {
    let query = query.to_lowercase();
    let target_lower = target.to_lowercase();

    // Exact prefix match is best
    if target_lower.starts_with(&query) {
        return 1000 + (100 - target.len() as i32).max(0);
    }

    // Contains match
    if target_lower.contains(&query) {
        return 500 + (100 - target.len() as i32).max(0);
    }

    // Fuzzy character match
    let mut score = 0;
    let mut query_chars = query.chars().peekable();
    let mut consecutive = 0;

    for c in target_lower.chars() {
        if query_chars.peek() == Some(&c) {
            query_chars.next();
            consecutive += 1;
            score += 10 + consecutive * 5;  // Bonus for consecutive matches
        } else {
            consecutive = 0;
        }
    }

    // All query chars must match
    if query_chars.peek().is_some() {
        return 0;
    }

    score
}

/// Parse a .desktop file
fn parse_desktop_file(path: &PathBuf) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut no_display = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }

        if line.starts_with('[') {
            in_desktop_entry = false;
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some(value) = line.strip_prefix("Name=") {
            if name.is_none() {  // Use first Name= only
                name = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("Exec=") {
            // Remove field codes like %f, %u, etc.
            let cleaned = value
                .replace("%f", "")
                .replace("%F", "")
                .replace("%u", "")
                .replace("%U", "")
                .replace("%c", "")
                .replace("%k", "")
                .trim()
                .to_string();
            exec = Some(cleaned);
        } else if let Some(value) = line.strip_prefix("Icon=") {
            icon = Some(value.to_string());
        } else if line == "NoDisplay=true" {
            no_display = true;
        }
    }

    if no_display {
        return None;
    }

    Some(AppEntry {
        name: name?,
        exec: exec?,
        icon,
        desktop_file: path.clone(),
        score: 0,
    })
}

/// Read battery percentage from sysfs
fn read_battery_percent() -> Option<u8> {
    let paths = [
        "/sys/class/power_supply/BAT0/capacity",
        "/sys/class/power_supply/BAT1/capacity",
    ];

    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(percent) = content.trim().parse() {
                return Some(percent);
            }
        }
    }

    None
}

/// Check if battery is charging
fn read_battery_charging() -> Option<bool> {
    let paths = [
        "/sys/class/power_supply/BAT0/status",
        "/sys/class/power_supply/BAT1/status",
    ];

    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Some(content.trim() == "Charging");
        }
    }

    None
}

/// Layout calculations for rendering
pub struct CommandCenterLayout {
    pub total_width: i32,
    pub total_height: i32,

    // Main container (centered)
    pub container_x: i32,
    pub container_y: i32,
    pub container_width: i32,
    pub container_height: i32,

    // Search bar
    pub search_x: i32,
    pub search_y: i32,
    pub search_width: i32,
    pub search_height: i32,

    // App grid
    pub apps_x: i32,
    pub apps_y: i32,
    pub apps_width: i32,
    pub apps_height: i32,
    pub app_card_width: i32,
    pub app_card_height: i32,
    pub app_columns: i32,

    // System info bar
    pub system_x: i32,
    pub system_y: i32,
    pub system_width: i32,
    pub system_height: i32,
}

impl CommandCenterLayout {
    pub fn calculate(screen_width: i32, screen_height: i32) -> Self {
        let container_width = (screen_width as f32 * 0.6).min(800.0) as i32;
        let container_height = (screen_height as f32 * 0.7).min(600.0) as i32;

        let container_x = (screen_width - container_width) / 2;
        let container_y = (screen_height - container_height) / 2;

        let padding = 24;
        let search_height = 56;
        let system_height = 48;

        Self {
            total_width: screen_width,
            total_height: screen_height,

            container_x,
            container_y,
            container_width,
            container_height,

            search_x: container_x + padding,
            search_y: container_y + padding,
            search_width: container_width - padding * 2,
            search_height,

            apps_x: container_x + padding,
            apps_y: container_y + padding + search_height + 16,
            apps_width: container_width - padding * 2,
            apps_height: container_height - padding * 2 - search_height - system_height - 32,
            app_card_width: 180,
            app_card_height: 64,
            app_columns: 3,

            system_x: container_x + padding,
            system_y: container_y + container_height - padding - system_height,
            system_width: container_width - padding * 2,
            system_height,
        }
    }
}
