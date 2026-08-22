//! Shared fixtures for unit tests. Available only under `cfg(test)`.

#![cfg(test)]

pub mod config;
pub mod devices;
pub mod transcript;

pub use config::sample_config;
pub use devices::sample_devices;
pub use transcript::sample_event;
