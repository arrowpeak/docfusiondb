use crate::config::LogConfig;
use crate::error::{DocFusionError, DocFusionResult};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize structured logging based on configuration
pub fn init_logging(config: &LogConfig) -> DocFusionResult<()> {
    let env_filter = EnvFilter::try_new(&config.level)
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| DocFusionError::config(format!("Invalid log level: {e}")))?;

    let registry = tracing_subscriber::registry().with(env_filter);

    match config.format.as_str() {
        "json" => {
            if let Some(file_path) = &config.file {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)?;

                registry.with(fmt::layer().json().with_writer(file)).init();
            } else {
                registry.with(fmt::layer().json()).init();
            }
        }
        "pretty" => {
            if let Some(file_path) = &config.file {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)?;

                registry
                    .with(fmt::layer().pretty().with_writer(file))
                    .init();
            } else {
                registry.with(fmt::layer().pretty()).init();
            }
        }
        "compact" => {
            if let Some(file_path) = &config.file {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)?;

                registry
                    .with(fmt::layer().compact().with_writer(file))
                    .init();
            } else {
                registry.with(fmt::layer().compact()).init();
            }
        }
        _ => {
            // Default to compact format
            if let Some(file_path) = &config.file {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)?;

                registry.with(fmt::layer().with_writer(file)).init();
            } else {
                registry.with(fmt::layer()).init();
            }
        }
    }

    Ok(())
}

/// Create a span for tracking database operations
#[macro_export]
macro_rules! db_span {
    ($operation:expr) => {
        tracing::info_span!("db_operation", operation = $operation)
    };
    ($operation:expr, $($key:expr => $value:expr),*) => {
        tracing::info_span!("db_operation", operation = $operation, $($key = $value),*)
    };
}

/// Create a span for tracking query operations
#[macro_export]
macro_rules! query_span {
    ($query:expr) => {
        tracing::info_span!("query", query = $query)
    };
    ($query:expr, $($key:expr => $value:expr),*) => {
        tracing::info_span!("query", query = $query, $($key = $value),*)
    };
}

/// Log performance metrics
#[macro_export]
macro_rules! log_performance {
    ($operation:expr, $duration:expr) => {
        tracing::info!(
            operation = $operation,
            duration_ms = $duration.as_millis(),
            "Performance metric"
        );
    };
    ($operation:expr, $duration:expr, $($key:expr => $value:expr),*) => {
        tracing::info!(
            operation = $operation,
            duration_ms = $duration.as_millis(),
            $($key = $value),*,
            "Performance metric"
        );
    };
}
