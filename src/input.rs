use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
    },
    input::{
        keyboard::{FilterResult, Keysym, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::seat::WaylandFocus,
};

use crate::config::SnapPosition;
use crate::state::VibeWM;
use crate::window::Direction;

/// Input handling state
pub struct InputState {
    /// Is resize mode active (mod+R held)?
    pub resize_mode: bool,

    /// Current pointer position
    pub pointer_pos: Point<f64, Logical>,

    /// Has quit been requested?
    pub quit_requested: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            resize_mode: false,
            pointer_pos: Point::from((0.0, 0.0)),
            quit_requested: false,
        }
    }
}

impl VibeWM {
    /// Process input events
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => self.handle_keyboard(event),
            InputEvent::PointerMotion { event } => self.handle_pointer_motion(event),
            InputEvent::PointerMotionAbsolute { event } => self.handle_pointer_motion_absolute(event),
            InputEvent::PointerButton { event } => self.handle_pointer_button(event),
            InputEvent::PointerAxis { event } => self.handle_pointer_axis(event),
            _ => {}
        }
    }

    fn handle_keyboard<I: InputBackend>(&mut self, event: impl KeyboardKeyEvent<I>) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = Event::time_msec(&event);
        let keycode = event.key_code();
        let pressed = event.state() == KeyState::Pressed;

        let keyboard = self.seat.get_keyboard().unwrap();

        keyboard.input::<(), _>(
            self,
            keycode,
            event.state(),
            serial,
            time,
            |state, modifiers, keysym_handle| {
                // Get the keysym from the handle
                let keysym = keysym_handle.modified_sym();

                if state.handle_keybind(modifiers, keysym, pressed) {
                    FilterResult::Intercept(())
                } else {
                    FilterResult::Forward
                }
            },
        );
    }

    /// Handle vibeWM keybinds - returns true if handled
    fn handle_keybind(&mut self, modifiers: &ModifiersState, keysym: Keysym, pressed: bool) -> bool {
        let mod_held = modifiers.logo;

        // Track resize mode (mod+R)
        if mod_held && keysym == Keysym::r {
            self.input.resize_mode = pressed;
            return true;
        }

        // Only handle on press, not release
        if !pressed {
            return false;
        }

        // Command center toggle always works
        if mod_held && keysym == Keysym::s {
            self.toggle_command_center();
            return true;
        }

        // When command center is open, route input there
        if self.command_center.visible {
            return self.handle_command_center_input(keysym, modifiers);
        }

        // Global quit
        if mod_held && keysym == Keysym::q {
            tracing::info!("Quit requested");
            self.input.quit_requested = true;
            return true;
        }

        if mod_held {
            match keysym {
                // Focus cycling: mod+Tab
                Keysym::Tab => {
                    if modifiers.shift {
                        self.windows.focus_prev();
                    } else {
                        self.windows.focus_next();
                    }
                    return true;
                }

                // Vim motions for move/resize: ijkl
                Keysym::i => {
                    self.handle_vim_motion(Direction::Up);
                    return true;
                }
                Keysym::k => {
                    self.handle_vim_motion(Direction::Down);
                    return true;
                }
                Keysym::j => {
                    self.handle_vim_motion(Direction::Left);
                    return true;
                }
                Keysym::l => {
                    self.handle_vim_motion(Direction::Right);
                    return true;
                }

                // Arrow keys for snap
                Keysym::Left => {
                    self.snap_focused(SnapPosition::Left);
                    return true;
                }
                Keysym::Right => {
                    self.snap_focused(SnapPosition::Right);
                    return true;
                }
                Keysym::Up => {
                    self.snap_focused(SnapPosition::Top);
                    return true;
                }
                Keysym::Down => {
                    self.snap_focused(SnapPosition::Bottom);
                    return true;
                }

                // Close window: mod+W
                Keysym::w => {
                    if let Some(window) = self.windows.focused() {
                        if let Some(toplevel) = window.toplevel() {
                            toplevel.send_close();
                        }
                    }
                    return true;
                }

                _ => {}
            }
        }

        false
    }

    /// Handle input when command center is open
    fn handle_command_center_input(&mut self, keysym: Keysym, _modifiers: &ModifiersState) -> bool {
        match keysym {
            // Close on Escape
            Keysym::Escape => {
                self.command_center.toggle();
                true
            }

            // Navigate with arrows
            Keysym::Up => {
                self.command_center.select_prev();
                true
            }
            Keysym::Down => {
                self.command_center.select_next();
                true
            }

            // Launch on Enter
            Keysym::Return => {
                if let Some(exec) = self.command_center.launch_selected() {
                    // Spawn the app
                    std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&exec)
                        .spawn()
                        .ok();
                }
                true
            }

            // Backspace for search
            Keysym::BackSpace => {
                self.command_center.handle_backspace();
                true
            }

            // Type to search - handle printable characters
            _ => {
                // Convert keysym to char if it's a printable character
                if let Some(c) = keysym_to_char(keysym) {
                    self.command_center.handle_char(c);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn handle_vim_motion(&mut self, direction: Direction) {
        if self.input.resize_mode {
            self.resize_focused(direction);
        } else {
            self.move_focused(direction);
        }
    }

    fn move_focused(&mut self, direction: Direction) {
        let Some(window) = self.windows.focused().cloned() else {
            return;
        };

        let Some(current_loc) = self.space.element_location(&window) else {
            return;
        };

        let delta = direction.to_delta(self.config.move_step);
        let new_loc = current_loc + delta;

        self.space.map_element(window, new_loc, false);
    }

    fn resize_focused(&mut self, direction: Direction) {
        let Some(window) = self.windows.focused() else {
            return;
        };

        let current_size = window.geometry().size;
        let (dw, dh) = direction.to_size_delta(self.config.resize_step);

        let new_width = (current_size.w + dw).max(100);
        let new_height = (current_size.h + dh).max(100);

        if let Some(toplevel) = window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.size = Some((new_width, new_height).into());
            });
            toplevel.send_pending_configure();
        }
    }

    fn snap_focused(&mut self, position: SnapPosition) {
        let Some(window) = self.windows.focused().cloned() else {
            return;
        };

        let output_size = self.output.as_ref()
            .and_then(|o| o.current_mode())
            .map(|m| m.size)
            .unwrap_or((1920, 1080).into());

        let gap = self.config.outer_gap;
        let inner = self.config.inner_gap;

        let (x, y, w, h) = match position {
            SnapPosition::Left => (
                gap,
                gap,
                output_size.w / 2 - gap - inner / 2,
                output_size.h - gap * 2,
            ),
            SnapPosition::Right => (
                output_size.w / 2 + inner / 2,
                gap,
                output_size.w / 2 - gap - inner / 2,
                output_size.h - gap * 2,
            ),
            SnapPosition::Top => (
                gap,
                gap,
                output_size.w - gap * 2,
                output_size.h / 2 - gap - inner / 2,
            ),
            SnapPosition::Bottom => (
                gap,
                output_size.h / 2 + inner / 2,
                output_size.w - gap * 2,
                output_size.h / 2 - gap - inner / 2,
            ),
            SnapPosition::TopLeft => (
                gap,
                gap,
                output_size.w / 2 - gap - inner / 2,
                output_size.h / 2 - gap - inner / 2,
            ),
            SnapPosition::TopRight => (
                output_size.w / 2 + inner / 2,
                gap,
                output_size.w / 2 - gap - inner / 2,
                output_size.h / 2 - gap - inner / 2,
            ),
            SnapPosition::BottomLeft => (
                gap,
                output_size.h / 2 + inner / 2,
                output_size.w / 2 - gap - inner / 2,
                output_size.h / 2 - gap - inner / 2,
            ),
            SnapPosition::BottomRight => (
                output_size.w / 2 + inner / 2,
                output_size.h / 2 + inner / 2,
                output_size.w / 2 - gap - inner / 2,
                output_size.h / 2 - gap - inner / 2,
            ),
            SnapPosition::Maximize => (
                gap,
                gap,
                output_size.w - gap * 2,
                output_size.h - gap * 2,
            ),
            SnapPosition::Center => {
                let current_size = window.geometry().size;
                (
                    (output_size.w - current_size.w) / 2,
                    (output_size.h - current_size.h) / 2,
                    current_size.w,
                    current_size.h,
                )
            }
        };

        // Move window
        self.space.map_element(window.clone(), (x, y), false);

        // Resize window
        if let Some(toplevel) = window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.size = Some((w, h).into());
            });
            toplevel.send_pending_configure();
        }
    }

    fn handle_pointer_motion<I: InputBackend>(&mut self, event: impl PointerMotionEvent<I>) {
        let delta = event.delta();
        self.input.pointer_pos += delta;

        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.seat.get_pointer().unwrap();

        // Find surface under pointer and convert to the right types
        let under = self.space
            .element_under(self.input.pointer_pos)
            .and_then(|(window, loc)| {
                window.wl_surface().map(|surface| {
                    (surface.into_owned(), loc.to_f64())
                })
            });

        pointer.motion(
            self,
            under,
            &MotionEvent {
                location: self.input.pointer_pos,
                serial,
                time: event.time_msec(),
            },
        );
    }

    fn handle_pointer_motion_absolute<I: InputBackend>(&mut self, event: impl AbsolutePositionEvent<I>) {
        let output_size = self.output.as_ref()
            .and_then(|o| o.current_mode())
            .map(|m| m.size)
            .unwrap_or((1920, 1080).into());

        self.input.pointer_pos = (
            event.x_transformed(output_size.w) as f64,
            event.y_transformed(output_size.h) as f64,
        ).into();

        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.seat.get_pointer().unwrap();

        let under = self.space
            .element_under(self.input.pointer_pos)
            .and_then(|(window, loc)| {
                window.wl_surface().map(|surface| {
                    (surface.into_owned(), loc.to_f64())
                })
            });

        pointer.motion(
            self,
            under,
            &MotionEvent {
                location: self.input.pointer_pos,
                serial,
                time: event.time_msec(),
            },
        );
    }

    fn handle_pointer_button<I: InputBackend>(&mut self, event: impl PointerButtonEvent<I>) {
        let serial = SERIAL_COUNTER.next_serial();
        let pointer = self.seat.get_pointer().unwrap();

        pointer.button(
            self,
            &ButtonEvent {
                button: event.button_code(),
                state: event.state(),
                serial,
                time: event.time_msec(),
            },
        );

        // Focus on click
        if event.state() == ButtonState::Pressed {
            if let Some((window, _)) = self.space.element_under(self.input.pointer_pos) {
                // Find window index and focus it
                let _pos = self.windows.all().iter().position(|w| w == window);
                // TODO: proper focus management
            }
        }
    }

    fn handle_pointer_axis<I: InputBackend>(&mut self, event: impl PointerAxisEvent<I>) {
        let pointer = self.seat.get_pointer().unwrap();

        let mut frame = AxisFrame::new(event.time_msec());

        if let Some(amount) = event.amount(Axis::Horizontal) {
            frame = frame.value(Axis::Horizontal, amount);
        }
        if let Some(amount) = event.amount(Axis::Vertical) {
            frame = frame.value(Axis::Vertical, amount);
        }

        if event.source() == AxisSource::Finger {
            frame = frame.source(AxisSource::Finger);
        }

        pointer.axis(self, frame);
    }
}

/// Convert keysym to character for text input
fn keysym_to_char(keysym: Keysym) -> Option<char> {
    // Handle common ASCII characters
    let raw = keysym.raw();

    // Lowercase letters (a-z)
    if raw >= 0x61 && raw <= 0x7a {
        return Some(raw as u8 as char);
    }

    // Uppercase letters (A-Z) - convert to lowercase for search
    if raw >= 0x41 && raw <= 0x5a {
        return Some((raw as u8 + 32) as char);
    }

    // Numbers (0-9)
    if raw >= 0x30 && raw <= 0x39 {
        return Some(raw as u8 as char);
    }

    // Space
    if raw == 0x20 {
        return Some(' ');
    }

    // Common punctuation
    match raw {
        0x2d => Some('-'),
        0x5f => Some('_'),
        0x2e => Some('.'),
        _ => None,
    }
}
