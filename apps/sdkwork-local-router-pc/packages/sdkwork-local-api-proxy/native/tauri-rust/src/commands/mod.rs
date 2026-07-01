pub const LOAD_CONFIG: &str = "localApiProxy.loadConfig";
pub const SAVE_CONFIG: &str = "localApiProxy.saveConfig";
pub const VALIDATE_CONFIG: &str = "localApiProxy.validateConfig";
pub const START_RUNTIME: &str = "localApiProxy.startRuntime";
pub const STOP_RUNTIME: &str = "localApiProxy.stopRuntime";
pub const RESTART_RUNTIME: &str = "localApiProxy.restartRuntime";
pub const GET_RUNTIME_STATUS: &str = "localApiProxy.getRuntimeStatus";
pub const PROBE_ROUTE: &str = "localApiProxy.probeRoute";
pub const LIST_REQUEST_LOGS: &str = "localApiProxy.listRequestLogs";
pub const LIST_MESSAGE_LOGS: &str = "localApiProxy.listMessageLogs";
pub const UPDATE_CAPTURE_SETTINGS: &str = "localApiProxy.updateCaptureSettings";

pub fn command_catalog() -> &'static [&'static str] {
    &[
        LOAD_CONFIG,
        SAVE_CONFIG,
        VALIDATE_CONFIG,
        START_RUNTIME,
        STOP_RUNTIME,
        RESTART_RUNTIME,
        GET_RUNTIME_STATUS,
        PROBE_ROUTE,
        LIST_REQUEST_LOGS,
        LIST_MESSAGE_LOGS,
        UPDATE_CAPTURE_SETTINGS,
    ]
}
