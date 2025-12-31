use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use smithay::{
    desktop::{Space, Window},
    input::{keyboard::XkbConfig, Seat, SeatHandler, SeatState},
    output::Output,
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle, Resource,
        },
    },
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
                set_data_device_focus,
            },
            SelectionHandler,
        },
        output::{OutputHandler, OutputManagerState},
        seat::WaylandFocus,
        shell::xdg::{XdgShellHandler, XdgShellState, ToplevelSurface, PopupSurface, PositionerState},
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};

use crate::config::Config;
use crate::window::WindowManager;
use crate::input::InputState;
use crate::command_center::CommandCenter;

/// Main compositor state
pub struct VibeWM {
    pub config: Config,
    pub start_time: Instant,
    pub display_handle: DisplayHandle,

    // Wayland state
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<Self>,
    pub seat: Seat<Self>,

    // Desktop
    pub space: Space<Window>,
    pub output: Option<Output>,

    // vibeWM specific
    pub windows: WindowManager,
    pub input: InputState,

    // Command center - the anti-suckless control panel
    pub command_center: CommandCenter,
}

impl VibeWM {
    pub fn new(event_loop: &mut EventLoop<'static, Self>, config: Config) -> Result<Self> {
        let display = Display::<Self>::new()?;
        let display_handle = display.handle();
        let loop_handle = event_loop.handle();

        // Initialize Wayland state
        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
        let data_device_state = DataDeviceState::new::<Self>(&display_handle);

        // Create seat
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display_handle, "vibeWM");

        // Add keyboard with default XKB config
        seat.add_keyboard(XkbConfig::default(), 200, 25)?;

        // Add pointer
        seat.add_pointer();

        // Create socket for clients to connect
        let socket = ListeningSocketSource::new_auto()?;
        let socket_name = socket.socket_name().to_string_lossy().to_string();
        tracing::info!("Wayland socket: {}", socket_name);

        // Set WAYLAND_DISPLAY env var
        std::env::set_var("WAYLAND_DISPLAY", &socket_name);

        // Add socket to event loop
        loop_handle.insert_source(socket, |client_stream, _, state| {
            state
                .display_handle
                .insert_client(client_stream, Arc::new(ClientState::default()))
                .ok();
        })?;

        // Add display to event loop
        loop_handle.insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            |_, display, state| {
                // SAFETY: we don't drop the display
                unsafe {
                    display.get_mut().dispatch_clients(state).ok();
                }
                Ok(PostAction::Continue)
            },
        )?;

        Ok(Self {
            config,
            start_time: Instant::now(),
            display_handle,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            data_device_state,
            seat_state,
            seat,
            space: Space::default(),
            output: None,
            windows: WindowManager::new(),
            input: InputState::new(),
            command_center: CommandCenter::new(),
        })
    }

    pub fn handle_pending(&mut self) {
        // Handle any pending compositor work
        self.space.refresh();
        self.windows.cleanup_closed();

        // Update command center animations
        self.command_center.update();

        // Flush client events
        self.display_handle.flush_clients().ok();
    }

    pub fn toggle_command_center(&mut self) {
        self.command_center.toggle();
    }
}

// Client state for connected Wayland clients
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

// SeatHandler implementation
impl SeatHandler for VibeWM {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let client = focused.and_then(|s| self.display_handle.get_client(s.id()).ok());
        set_data_device_focus(&self.display_handle, seat, client);
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {
        // Handle cursor changes
    }
}

// OutputHandler implementation
impl OutputHandler for VibeWM {}

// Delegate implementations for Wayland protocols
impl CompositorHandler for VibeWM {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a smithay::reexports::wayland_server::Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        // Handle surface commit - find window with this surface
        let window = self.space.elements()
            .find(|w| w.wl_surface().map(|s| &*s == surface).unwrap_or(false))
            .cloned();

        if let Some(window) = window {
            window.on_commit();
        }
    }
}

impl BufferHandler for VibeWM {
    fn buffer_destroyed(&mut self, _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer) {}
}

impl ShmHandler for VibeWM {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl XdgShellHandler for VibeWM {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface);

        // Center new windows
        let size = self.output.as_ref()
            .map(|o| o.current_mode().map(|m| m.size).unwrap_or((1920, 1080).into()))
            .unwrap_or((1920, 1080).into());

        let window_size = window.geometry().size;
        let x = (size.w - window_size.w) / 2;
        let y = (size.h - window_size.h) / 2;

        self.space.map_element(window.clone(), (x, y), false);
        self.windows.add(window);

        tracing::info!("New window mapped");
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // Handle popups
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        // Find and remove the window
        let window = self.space.elements()
            .find(|w| w.toplevel().map(|t| t == &surface).unwrap_or(false))
            .cloned();

        if let Some(window) = window {
            self.space.unmap_elem(&window);
            self.windows.remove(&window);
        }
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat, _serial: Serial) {}
    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {}
}

impl SelectionHandler for VibeWM {
    type SelectionUserData = ();
}

impl DataDeviceHandler for VibeWM {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for VibeWM {}
impl ServerDndGrabHandler for VibeWM {}

smithay::delegate_compositor!(VibeWM);
smithay::delegate_shm!(VibeWM);
smithay::delegate_xdg_shell!(VibeWM);
smithay::delegate_data_device!(VibeWM);
smithay::delegate_output!(VibeWM);
smithay::delegate_seat!(VibeWM);
