use std::fmt::Display;

use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn trace(event: &str, extra: &[(&str, String)]) {
    if extra.is_empty() {
        info!(event);
    } else {
        info!(event, fields = ?extra);
    }
}

pub fn trace_error(event: &str, error: &dyn Display, extra: &[(&str, String)]) {
    error!(event, err = %error, fields = ?extra);
}

pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry().with(filter).with(fmt::layer()).init();
}
