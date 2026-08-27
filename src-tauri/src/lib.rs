mod db;
mod error;
mod state;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let pool = db::init_pool(&data_dir)?;
            app.manage(AppState { db: pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running forgeyard");
}
