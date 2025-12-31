//! Backend initialization for vibeWM
//!
//! Winit backend for development/testing (runs in a window)

use std::time::Duration;

use anyhow::Result;
use smithay::{
    backend::{
        renderer::{
            element::surface::WaylandSurfaceRenderElement,
            glow::GlowRenderer,
            Frame, Renderer,
        },
        winit::{self, WinitEvent},
    },
    desktop::space::SpaceRenderElements,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Physical, Rectangle, Transform},
};

use crate::state::VibeWM;

/// Run vibeWM with the winit backend (windowed mode)
pub fn run_winit(event_loop: &mut EventLoop<'static, VibeWM>, state: &mut VibeWM) -> Result<()> {
    // Create winit backend
    let (mut backend, mut winit_event_loop) = winit::init::<GlowRenderer>()
        .map_err(|e| anyhow::anyhow!("Failed to init winit backend: {:?}", e))?;

    // Get output size from the window
    let size = backend.window_size();

    // Create output
    let mode = Mode {
        size: (size.w as i32, size.h as i32).into(),
        refresh: 60_000, // 60 Hz
    };

    let output = Output::new(
        "vibeWM-winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(), // Unknown physical size
            subpixel: Subpixel::Unknown,
            make: "vibeWM".to_string(),
            model: "Winit Window".to_string(),
        },
    );

    output.create_global::<VibeWM>(&state.display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    state.output = Some(output.clone());
    state.space.map_output(&output, (0, 0));

    tracing::info!("Winit backend initialized: {}x{}", size.w, size.h);

    // Insert winit event source into the event loop
    let mut running = true;

    while running {
        // Process winit events
        let pump_status = winit_event_loop.dispatch_new_events(|event| {
            match event {
                WinitEvent::Resized { size, .. } => {
                    let mode = Mode {
                        size: (size.w as i32, size.h as i32).into(),
                        refresh: 60_000,
                    };
                    output.change_current_state(Some(mode), None, None, None);
                }
                WinitEvent::Input(event) => {
                    state.process_input_event(event);
                }
                WinitEvent::Focus(_) => {}
                WinitEvent::Redraw => {}
                WinitEvent::CloseRequested => {
                    running = false;
                }
            }
        });

        use smithay::reexports::winit::platform::pump_events::PumpStatus;
        if matches!(pump_status, PumpStatus::Exit(_)) || state.input.quit_requested {
            running = false;
        }

        // Render frame
        let size = backend.window_size();
        let bg = state.config.colors.background;

        // Bind the backend - returns renderer and framebuffer target
        let (renderer, mut target) = backend.bind()
            .map_err(|e| anyhow::anyhow!("Bind error: {:?}", e))?;

        // Get render elements from the space
        let output_ref = state.output.as_ref().unwrap();
        let _elements: Vec<SpaceRenderElements<GlowRenderer, WaylandSurfaceRenderElement<GlowRenderer>>> =
            state.space.render_elements_for_output(renderer, output_ref, 1.0)
                .map_err(|e| anyhow::anyhow!("Render elements error: {:?}", e))?;

        // Render frame
        let frame_size = (size.w as i32, size.h as i32).into();
        let damage = Rectangle::<i32, Physical>::from_size(frame_size);

        // Start a render pass with the target
        let mut frame = renderer.render(&mut target, frame_size, Transform::Normal)
            .map_err(|e| anyhow::anyhow!("Render start error: {:?}", e))?;

        // Clear with background color
        frame.clear(bg.into(), &[damage])
            .map_err(|e| anyhow::anyhow!("Clear error: {:?}", e))?;

        // TODO: Actually render elements to the frame
        // This requires iterating elements and calling draw on each

        // Finish the frame (ignore SyncPoint - we don't need fence synchronization for basic rendering)
        let _ = frame.finish()
            .map_err(|e| anyhow::anyhow!("Frame finish error: {:?}", e))?;

        // Drop target before submit
        drop(target);

        // Submit the frame
        backend.submit(None)
            .map_err(|e| anyhow::anyhow!("Submit error: {:?}", e))?;

        // Handle pending compositor work
        state.handle_pending();

        // Dispatch event loop
        event_loop.dispatch(Duration::from_millis(16), state)?;
    }

    Ok(())
}

// TODO: Command center overlay rendering
// Will need custom RenderElement implementation for the overlay
// For now, command center state exists but isn't rendered
