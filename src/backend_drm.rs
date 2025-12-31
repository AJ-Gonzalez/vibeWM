//! DRM/KMS backend for vibeWM
//!
//! This backend runs directly on hardware - no window, owns the whole display.
//! Used for bare metal or VM without a desktop environment.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use smithay::{
    backend::{
        allocator::{
            dmabuf::Dmabuf,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            compositor::DrmCompositor, CreateDrmNodeError, DrmDevice, DrmDeviceFd, DrmError,
            DrmEvent, DrmNode, GbmBufferedSurface, NodeType,
        },
        egl::{EGLContext, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            damage::OutputDamageTracker,
            element::surface::WaylandSurfaceRenderElement,
            glow::GlowRenderer,
            multigpu::{gbm::GbmGlesBackend, GpuManager},
            Bind, Frame, Renderer,
        },
        session::{libseat::LibSeatSession, Session, Event as SessionEvent},
        udev::{self, UdevBackend, UdevEvent},
    },
    desktop::space::SpaceRenderElements,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop, LoopHandle, RegistrationToken,
        },
        drm::control::{connector, crtc, ModeTypeFlags},
        input::Libinput,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{DeviceFd, Physical, Rectangle, Transform},
    wayland::dmabuf::DmabufState,
};

use crate::state::VibeWM;

/// DRM backend state
pub struct DrmBackendData {
    pub session: LibSeatSession,
    pub primary_gpu: DrmNode,
    pub gpus: HashMap<DrmNode, GpuData>,
}

/// Per-GPU data
pub struct GpuData {
    pub device: DrmDevice,
    pub gbm: GbmDevice<DrmDeviceFd>,
    pub renderer: GlowRenderer,
    pub surfaces: HashMap<crtc::Handle, SurfaceData>,
    pub registration_token: RegistrationToken,
}

/// Per-output surface data
pub struct SurfaceData {
    pub output: Output,
    pub compositor: GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, ()>,
    pub damage_tracker: OutputDamageTracker,
}

/// Run vibeWM with the DRM backend (bare metal mode)
pub fn run_drm(event_loop: &mut EventLoop<'static, VibeWM>, state: &mut VibeWM) -> Result<()> {
    tracing::info!("Initializing DRM backend...");

    // Initialize session (libseat handles permissions)
    let (session, notifier) = LibSeatSession::new()
        .context("Failed to create libseat session - are you running from a TTY?")?;

    tracing::info!("Session opened on seat: {}", session.seat());

    // Add session to event loop
    event_loop
        .handle()
        .insert_source(notifier, |event, _, _state| match event {
            SessionEvent::ActivateSession => {
                tracing::info!("Session activated");
            }
            SessionEvent::PauseSession => {
                tracing::info!("Session paused");
            }
        })
        .context("Failed to insert session source")?;

    // Initialize udev for device discovery
    let udev_backend = UdevBackend::new(session.seat())
        .context("Failed to create udev backend")?;

    // Find primary GPU
    let primary_gpu = udev::primary_gpu(session.seat())
        .context("Failed to find primary GPU")?
        .and_then(|p| DrmNode::from_path(&p).ok()?.node_with_type(NodeType::Render)?.ok())
        .context("No suitable GPU found")?;

    tracing::info!("Primary GPU: {:?}", primary_gpu);

    // Initialize libinput for input handling
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        session.clone().into(),
    );
    libinput_context
        .udev_assign_seat(session.seat())
        .map_err(|_| anyhow::anyhow!("Failed to assign seat to libinput"))?;

    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    // Add libinput to event loop
    event_loop
        .handle()
        .insert_source(libinput_backend, |event, _, state| {
            state.process_input_event(event);
        })
        .context("Failed to insert libinput source")?;

    // Process existing GPUs
    let mut gpus: HashMap<DrmNode, GpuData> = HashMap::new();

    for (device_id, path) in udev_backend.device_list() {
        if let Err(e) = add_gpu(event_loop.handle(), &session, &mut gpus, state, &path, primary_gpu) {
            tracing::warn!("Failed to add GPU {:?}: {:?}", device_id, e);
        }
    }

    // Add udev to event loop for hotplug
    event_loop
        .handle()
        .insert_source(udev_backend, move |event, _, state| match event {
            UdevEvent::Added { device_id, path } => {
                tracing::info!("GPU added: {:?}", device_id);
                // Would need to pass gpus map here for real hotplug support
            }
            UdevEvent::Changed { device_id } => {
                tracing::info!("GPU changed: {:?}", device_id);
            }
            UdevEvent::Removed { device_id } => {
                tracing::info!("GPU removed: {:?}", device_id);
            }
        })
        .context("Failed to insert udev source")?;

    // Set up render timer (60 FPS)
    let timer = Timer::immediate();
    event_loop
        .handle()
        .insert_source(timer, |_, _, state| {
            // Render all outputs
            // This would iterate through gpus and render each surface
            state.handle_pending();
            TimeoutAction::ToDuration(Duration::from_millis(16))
        })
        .context("Failed to insert render timer")?;

    tracing::info!("DRM backend ready");

    // Main loop
    let mut running = true;
    while running {
        event_loop
            .dispatch(Duration::from_millis(16), state)
            .context("Event loop error")?;

        // Check for quit
        if state.input.quit_requested {
            running = false;
        }
    }

    Ok(())
}

/// Add a GPU to the compositor
fn add_gpu(
    handle: LoopHandle<'static, VibeWM>,
    session: &LibSeatSession,
    gpus: &mut HashMap<DrmNode, GpuData>,
    state: &mut VibeWM,
    path: &Path,
    primary_gpu: DrmNode,
) -> Result<()> {
    let node = DrmNode::from_path(path)
        .context("Failed to get DRM node")?;

    // Open DRM device
    let fd = session
        .open(
            path,
            smithay::backend::session::OFlag::O_RDWR
                | smithay::backend::session::OFlag::O_CLOEXEC
                | smithay::backend::session::OFlag::O_NOCTTY
                | smithay::backend::session::OFlag::O_NONBLOCK,
        )
        .context("Failed to open DRM device")?;

    let device_fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let (drm, drm_notifier) = DrmDevice::new(device_fd.clone(), true)
        .context("Failed to create DRM device")?;

    // Create GBM device
    let gbm = GbmDevice::new(device_fd)
        .context("Failed to create GBM device")?;

    // Create EGL display and renderer
    let egl_display = unsafe { EGLDisplay::new(gbm.clone()) }
        .context("Failed to create EGL display")?;

    let egl_context = EGLContext::new(&egl_display)
        .context("Failed to create EGL context")?;

    let renderer = unsafe { GlowRenderer::new(egl_context) }
        .context("Failed to create Glow renderer")?;

    // Scan connectors and create outputs
    let mut surfaces = HashMap::new();

    for connector in drm
        .resource_handles()
        .context("Failed to get DRM resources")?
        .connectors()
    {
        let connector_info = drm
            .get_connector(*connector, true)
            .context("Failed to get connector info")?;

        if connector_info.state() != connector::State::Connected {
            continue;
        }

        // Find best mode (prefer native/preferred)
        let mode = connector_info
            .modes()
            .iter()
            .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
            .or_else(|| connector_info.modes().first())
            .context("No modes available for connector")?;

        tracing::info!(
            "Found display: {:?} @ {}x{}",
            connector_info.interface(),
            mode.size().0,
            mode.size().1
        );

        // Find a CRTC for this connector
        let encoder = connector_info
            .current_encoder()
            .and_then(|e| drm.get_encoder(e).ok())
            .context("No encoder for connector")?;

        let crtc = encoder.crtc().context("No CRTC for encoder")?;

        // Create output
        let output_mode = Mode {
            size: (mode.size().0 as i32, mode.size().1 as i32).into(),
            refresh: (mode.vrefresh() * 1000) as i32,
        };

        let output = Output::new(
            format!("{:?}-{}", connector_info.interface(), connector_info.interface_id()),
            PhysicalProperties {
                size: (connector_info.size().unwrap_or((0, 0)).0 as i32,
                       connector_info.size().unwrap_or((0, 0)).1 as i32).into(),
                subpixel: Subpixel::Unknown,
                make: "vibeWM".into(),
                model: "DRM Output".into(),
            },
        );

        output.create_global::<VibeWM>(&state.display_handle);
        output.change_current_state(
            Some(output_mode),
            Some(Transform::Normal),
            None,
            Some((0, 0).into()),
        );
        output.set_preferred(output_mode);

        state.space.map_output(&output, (0, 0));
        state.output = Some(output.clone());

        // Create GBM surface for this output
        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );

        let surface = drm
            .create_surface(crtc, *mode, &[*connector])
            .context("Failed to create DRM surface")?;

        let compositor = GbmBufferedSurface::new(
            surface,
            allocator,
            &[Fourcc::Argb8888, Fourcc::Xrgb8888],
        )
        .context("Failed to create GBM buffered surface")?;

        let damage_tracker = OutputDamageTracker::from_output(&output);

        surfaces.insert(crtc, SurfaceData {
            output,
            compositor,
            damage_tracker,
        });
    }

    // Add DRM events to event loop
    let token = handle
        .insert_source(drm_notifier, |event, _, state| match event {
            DrmEvent::VBlank(crtc) => {
                // Frame complete, can render next
            }
            DrmEvent::Error(e) => {
                tracing::error!("DRM error: {:?}", e);
            }
        })
        .context("Failed to insert DRM source")?;

    gpus.insert(node, GpuData {
        device: drm,
        gbm,
        renderer,
        surfaces,
        registration_token: token,
    });

    Ok(())
}

/// Render a frame on a specific output
fn render_frame(
    gpu: &mut GpuData,
    crtc: crtc::Handle,
    state: &mut VibeWM,
) -> Result<()> {
    let surface_data = gpu.surfaces.get_mut(&crtc).context("No surface for CRTC")?;
    let output = &surface_data.output;

    // Get render elements
    let elements: Vec<SpaceRenderElements<GlowRenderer, WaylandSurfaceRenderElement<GlowRenderer>>> =
        state.space.render_elements_for_output(&mut gpu.renderer, output, 1.0)
            .map_err(|e| anyhow::anyhow!("Failed to get render elements: {:?}", e))?;

    // Render
    let bg = state.config.colors.background;

    surface_data.compositor.queue_buffer(|buffer| {
        gpu.renderer.bind(buffer)?;

        let size = output.current_mode().unwrap().size;
        let frame_size = (size.w, size.h).into();
        let damage = Rectangle::<i32, Physical>::from_size(frame_size);

        let mut frame = gpu.renderer.render(frame_size, Transform::Normal)?;
        frame.clear(bg.into(), &[damage])?;

        // TODO: Draw elements

        let _ = frame.finish()?;
        Ok(())
    })?;

    // Submit to display
    surface_data.compositor.frame_submitted()?;

    Ok(())
}
