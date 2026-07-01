use std::path::{Path, PathBuf};

const SDKWORK_STATE_DIRECTORY_NAME: &str = ".sdkwork";
const SDKWORK_CONFIG_FILE_NAME: &str = "sdkwork.json";
const SDKWORK_WORKSPACE_DIRECTORY_NAME: &str = "workspace";
const HERMES_STATE_DIRECTORY_NAME: &str = ".hermes";
const HERMES_CONFIG_FILE_NAME: &str = "config.yaml";
const OPENCLAW_STATE_DIRECTORY_NAME: &str = ".openclaw";
const OPENCLAW_CONFIG_FILE_NAME: &str = "openclaw.json";

pub fn build_standard_sdkwork_root_dir(user_root: &Path) -> PathBuf {
    user_root.join(SDKWORK_STATE_DIRECTORY_NAME)
}

pub fn build_standard_sdkwork_config_file_path(user_root: &Path) -> PathBuf {
    build_standard_sdkwork_root_dir(user_root).join(SDKWORK_CONFIG_FILE_NAME)
}

pub fn build_standard_sdkwork_workspace_dir(user_root: &Path) -> PathBuf {
    build_standard_sdkwork_root_dir(user_root).join(SDKWORK_WORKSPACE_DIRECTORY_NAME)
}

pub fn build_standard_hermes_root_dir(user_root: &Path) -> PathBuf {
    user_root.join(HERMES_STATE_DIRECTORY_NAME)
}

pub fn build_standard_hermes_config_file_path(user_root: &Path) -> PathBuf {
    build_standard_hermes_root_dir(user_root).join(HERMES_CONFIG_FILE_NAME)
}

/// Build the standard OpenClaw state root directory under the user root.
///
/// This is `~/.sdkwork/<product>/.openclaw` — the OpenClaw kernel's
/// state directory, distinct from the generic SDKWork state directory.
pub fn build_standard_openclaw_root_dir(user_root: &Path) -> PathBuf {
    user_root.join(OPENCLAW_STATE_DIRECTORY_NAME)
}

/// Build the standard OpenClaw config file path under the user root.
pub fn build_standard_openclaw_config_file_path(user_root: &Path) -> PathBuf {
    build_standard_openclaw_root_dir(user_root).join(OPENCLAW_CONFIG_FILE_NAME)
}

/// Build the standard OpenClaw workspace directory under the user root.
pub fn build_standard_openclaw_workspace_dir(user_root: &Path) -> PathBuf {
    build_standard_openclaw_root_dir(user_root).join(SDKWORK_WORKSPACE_DIRECTORY_NAME)
}
