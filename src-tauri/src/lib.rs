mod commands;

use commands::{
    autodiscovery::autodiscover_repos,
    config::{add_repo, detect_install_scripts, get_config, remove_repo, set_config},
    git::{get_repo_status, git_pull, test_git_connection},
    jars::{list_jars, run_install_jars},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            // config
            get_config,
            set_config,
            add_repo,
            remove_repo,
            detect_install_scripts,
            // autodiscovery
            autodiscover_repos,
            // git
            get_repo_status,
            git_pull,
            test_git_connection,
            // jars
            list_jars,
            run_install_jars,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
