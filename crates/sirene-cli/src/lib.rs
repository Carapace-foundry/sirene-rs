#![deny(unsafe_code)]

pub mod args;
pub mod commands;
pub mod error;
pub mod ingest;

pub use args::{Cli, Commands};
pub use error::{CliError, Result};
