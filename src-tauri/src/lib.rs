mod commands;
mod db;
mod deps;
mod detect;
mod error;
mod event_sink;
mod process;
mod project;
mod state;

use tauri::Manager;

use process::ProcessManager;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let pool = db::init_pool(&data_dir)?;
            let processes = ProcessManager::new(app.handle().clone());
            app.manage(AppState { db: pool, processes });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::add_project,
            commands::remove_project,
            commands::get_project_health,
            commands::list_dependencies,
            commands::start_process,
            commands::stop_process,
            commands::kill_process,
            commands::restart_process,
            commands::list_processes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running forgeyard");
}
