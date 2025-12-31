//! DRM/KMS backend for vibeWM
//!
//! This backend runs directly on hardware - no window, owns the whole display.
//! Used for bare metal or VM without a desktop environment.

use std::time::Duration;

use anyhow::{Context, Result};
use smithay::{
    backend::{
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        session::{libseat::LibSeatSession, Session, Event as SessionEvent},
        udev::{self, UdevBackend, UdevEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        input::Libinput,
    },
    utils::Transform,
};

use crate::state::VibeWM;

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
        .map_err(|e| anyhow::anyhow!("Failed to insert session source: {:?}", e))?;

    // Initialize udev for device discovery
    let udev_backend = UdevBackend::new(session.seat())
        .context("Failed to create udev backend")?;

    // Find primary GPU
    let primary_gpu = udev::primary_gpu(session.seat())
        .context("Failed to find primary GPU")?
        .context("No GPU found")?;

    tracing::info!("Primary GPU: {:?}", primary_gpu);

    // Initialize libinput for input handling
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        session.clone().into(),
    );
    libinput_context
        .udev_assign_seat(&session.seat())
        .map_err(|_| anyhow::anyhow!("Failed to assign seat to libinput"))?;

    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    // Add libinput to event loop
    event_loop
        .handle()
        .insert_source(libinput_backend, |event, _, state| {
            state.process_input_event(event);
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert libinput source: {:?}", e))?;

    // Log discovered GPUs
    for (device_id, path) in udev_backend.device_list() {
        tracing::info!("Found GPU: {:?} at {:?}", device_id, path);
    }

    // Add udev to event loop for hotplug
    event_loop
        .handle()
        .insert_source(udev_backend, move |event, _, _state| match event {
            UdevEvent::Added { device_id, path } => {
                tracing::info!("GPU added: {:?} at {:?}", device_id, path);
            }
            UdevEvent::Changed { device_id } => {
                tracing::info!("GPU changed: {:?}", device_id);
            }
            UdevEvent::Removed { device_id } => {
                tracing::info!("GPU removed: {:?}", device_id);
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert udev source: {:?}", e))?;

    // Create a dummy output for now
    // TODO: Actually enumerate DRM outputs and create real ones
    let output = Output::new(
        "DRM-1".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "vibeWM".into(),
            model: "Virtual Output".into(),
        },
    );

    let mode = Mode {
        size: (1920, 1080).into(),
        refresh: 60_000,
    };

    output.create_global::<VibeWM>(&state.display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    state.space.map_output(&output, (0, 0));
    state.output = Some(output);

    // Set up render timer (60 FPS)
    let timer = Timer::immediate();
    event_loop
        .handle()
        .insert_source(timer, |_, _, state| {
            // TODO: Actually render to DRM output
            state.handle_pending();
            TimeoutAction::ToDuration(Duration::from_millis(16))
        })
        .map_err(|e| anyhow::anyhow!("Failed to insert render timer: {:?}", e))?;

    tracing::info!("DRM backend ready (stub mode - no actual rendering yet)");
    tracing::warn!("DRM rendering not yet implemented - you'll see a blank screen");
    tracing::info!("Press mod+Q to quit");

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
