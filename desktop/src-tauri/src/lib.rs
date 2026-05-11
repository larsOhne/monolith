mod commands;
mod vault;
mod vault_registry;
mod worker;

use std::sync::{Arc, Mutex};

pub struct AppState {
    pub worker: Arc<Mutex<Option<Arc<worker::WorkerHandle>>>>,
}

impl AppState {
    fn new() -> Self {
        Self { worker: Arc::new(Mutex::new(None)) }
    }
}

pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::list_vaults,
            commands::add_vault,
            commands::remove_vault,
            commands::browse_directory,
            commands::read_vault_file,
            commands::write_vault_file,
            commands::vault_tree,
            commands::create_note,
            commands::delete_entry,
            commands::rename_entry,
            commands::start_backend,
            commands::get_backend_port,
            commands::stop_backend,
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Worker is dropped with AppState on app exit
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Monolith");
}
