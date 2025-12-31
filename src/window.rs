use std::collections::HashMap;

use smithay::{
    desktop::Window,
    utils::{IsAlive, Logical, Point, Rectangle},
};

use crate::config::SnapPosition;

/// Manages window state and operations
pub struct WindowManager {
    /// All managed windows in stacking order (bottom to top)
    windows: Vec<Window>,

    /// Currently focused window index
    focused: Option<usize>,

    /// Window metadata
    metadata: HashMap<u64, WindowMeta>,

    /// Counter for window IDs
    next_id: u64,
}

/// Metadata for each window
#[derive(Debug, Clone)]
pub struct WindowMeta {
    pub id: u64,

    /// Position before snapping (for restore)
    pub pre_snap_geometry: Option<Rectangle<i32, Logical>>,

    /// Current snap state
    pub snap_state: Option<SnapPosition>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            focused: None,
            metadata: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add(&mut self, window: Window) {
        let id = self.next_id;
        self.next_id += 1;

        self.metadata.insert(id, WindowMeta {
            id,
            pre_snap_geometry: None,
            snap_state: None,
        });

        self.windows.push(window);

        // Focus the new window
        self.focused = Some(self.windows.len() - 1);
    }

    pub fn remove(&mut self, window: &Window) {
        if let Some(pos) = self.windows.iter().position(|w| w == window) {
            self.windows.remove(pos);

            // Adjust focus
            if let Some(focused) = self.focused {
                if focused >= self.windows.len() {
                    self.focused = if self.windows.is_empty() {
                        None
                    } else {
                        Some(self.windows.len() - 1)
                    };
                } else if focused > pos {
                    self.focused = Some(focused - 1);
                }
            }
        }
    }

    pub fn focused(&self) -> Option<&Window> {
        self.focused.and_then(|i| self.windows.get(i))
    }

    pub fn focused_mut(&mut self) -> Option<&mut Window> {
        self.focused.and_then(|i| self.windows.get_mut(i))
    }

    pub fn focus_next(&mut self) {
        if self.windows.is_empty() {
            return;
        }

        self.focused = Some(match self.focused {
            Some(i) => (i + 1) % self.windows.len(),
            None => 0,
        });
    }

    pub fn focus_prev(&mut self) {
        if self.windows.is_empty() {
            return;
        }

        self.focused = Some(match self.focused {
            Some(i) => {
                if i == 0 {
                    self.windows.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        });
    }

    pub fn raise_focused(&mut self) {
        if let Some(i) = self.focused {
            if i < self.windows.len() - 1 {
                let window = self.windows.remove(i);
                self.windows.push(window);
                self.focused = Some(self.windows.len() - 1);
            }
        }
    }

    pub fn cleanup_closed(&mut self) {
        // Remove any windows that are no longer alive
        self.windows.retain(|w| w.alive());

        // Adjust focus if needed
        if let Some(focused) = self.focused {
            if focused >= self.windows.len() {
                self.focused = if self.windows.is_empty() {
                    None
                } else {
                    Some(self.windows.len() - 1)
                };
            }
        }
    }

    pub fn all(&self) -> &[Window] {
        &self.windows
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }
}

/// Direction for window operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Up,    // i / k
    Down,  // k / j
    Left,  // j / h
    Right, // l
}

impl Direction {
    /// Convert to movement delta
    pub fn to_delta(&self, step: i32) -> Point<i32, Logical> {
        match self {
            Direction::Up => Point::from((0, -step)),
            Direction::Down => Point::from((0, step)),
            Direction::Left => Point::from((-step, 0)),
            Direction::Right => Point::from((step, 0)),
        }
    }

    /// Convert to size delta for resizing
    pub fn to_size_delta(&self, step: i32) -> (i32, i32) {
        match self {
            Direction::Up => (0, -step),
            Direction::Down => (0, step),
            Direction::Left => (-step, 0),
            Direction::Right => (step, 0),
        }
    }
}
