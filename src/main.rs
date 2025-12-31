mod state;
mod input;
mod window;
mod config;
mod render;
mod command_center;
mod render_command_center;

// Backend modules - winit for dev, DRM for bare metal
#[cfg(not(feature = "udev"))]
mod backend;
#[cfg(feature = "udev")]
mod backend_drm;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use smithay::reexports::calloop::EventLoop;
use crate::state::VibeWM;
use crate::config::Config;

fn main() -> Result<()> {
    // Set up logging - vibecode style
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("vibeWM starting up ~");
    info!("  mod+ijkl: move windows");
    info!("  mod+R+ijkl: resize windows");
    info!("  mod+arrows: snap to halves");
    info!("  mod+S: command center");
    info!("  mod+W: close window");
    info!("  mod+Q: quit");

    let config = Config::default();

    // Create event loop with 'static lifetime
    let mut event_loop: EventLoop<'static, VibeWM> = EventLoop::try_new()?;

    // Initialize compositor state
    let mut state = VibeWM::new(&mut event_loop, config)?;

    info!("vibeWM ready - let's go ~");

    // Run with appropriate backend
    #[cfg(not(feature = "udev"))]
    {
        info!("Using winit backend (windowed mode)");
        backend::run_winit(&mut event_loop, &mut state)?;
    }

    #[cfg(feature = "udev")]
    {
        info!("Using DRM backend (bare metal mode)");
        backend_drm::run_drm(&mut event_loop, &mut state)?;
    }

    info!("vibeWM shutting down ~");
    Ok(())
}
