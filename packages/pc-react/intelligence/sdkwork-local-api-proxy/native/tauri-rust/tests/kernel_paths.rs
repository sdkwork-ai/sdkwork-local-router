use std::path::Path;

use sdkwork_local_api_proxy_native::kernel::{
    build_standard_hermes_config_file_path, build_standard_hermes_root_dir,
    build_standard_sdkwork_config_file_path, build_standard_sdkwork_root_dir,
    build_standard_sdkwork_workspace_dir,
};

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn sdkwork_kernel_paths_use_the_standard_user_root_layout() {
    let user_root = Path::new("C:/Users/admin/.sdkwork/crawstudio");

    assert_eq!(
        normalize(&build_standard_sdkwork_root_dir(user_root)),
        "C:/Users/admin/.sdkwork/crawstudio/.sdkwork"
    );
    assert_eq!(
        normalize(&build_standard_sdkwork_config_file_path(user_root)),
        "C:/Users/admin/.sdkwork/crawstudio/.sdkwork/sdkwork.json"
    );
    assert_eq!(
        normalize(&build_standard_sdkwork_workspace_dir(user_root)),
        "C:/Users/admin/.sdkwork/crawstudio/.sdkwork/workspace"
    );
}

#[test]
fn hermes_kernel_paths_use_the_standard_user_root_layout() {
    let user_root = Path::new("/Users/admin/.sdkwork/crawstudio");

    assert_eq!(
        normalize(&build_standard_hermes_root_dir(user_root)),
        "/Users/admin/.sdkwork/crawstudio/.hermes"
    );
    assert_eq!(
        normalize(&build_standard_hermes_config_file_path(user_root)),
        "/Users/admin/.sdkwork/crawstudio/.hermes/config.yaml"
    );
}
