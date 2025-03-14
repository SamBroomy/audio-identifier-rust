use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_subscriber() {
    // Create a Chrome trace file
    let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        // .with(
        //     fmt::layer()
        //         .with_timer(fmt::time::uptime()) // Add this timer
        //         .with_span_events(fmt::format::FmtSpan::CLOSE), // Add this to log span closures with durations
        // )
        .with(fmt::layer())
        .with(chrome_layer) // Add the Chrome layer
        .init();
}
