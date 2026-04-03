pub mod app;
pub mod i18n;
pub mod model;
pub mod services;
pub mod ui;

use tracing_subscriber::EnvFilter;

pub fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new(
                    "frostscan=debug,iced=error,iced_winit=error,iced_wgpu=error,wgpu=error,naga=error",
                )
            }),
        )
        .try_init();
}
