pub mod commands;
pub mod config;
pub mod constants;
pub mod error;
pub mod kernel;
pub mod observability;
pub mod probe;
pub mod response;
pub mod runtime;
pub mod snapshot;
pub mod streaming;
pub mod support;
pub mod translation;
pub mod upstream;

pub const NATIVE_BOUNDARY_NAME: &str = "sdkwork-local-api-proxy-native";

pub fn package_boundary_name() -> &'static str {
    NATIVE_BOUNDARY_NAME
}
