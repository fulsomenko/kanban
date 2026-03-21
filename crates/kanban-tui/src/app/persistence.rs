use kanban_core::AppConfig;

pub struct PersistenceState {
    pub save_file: Option<String>,
    pub app_config: AppConfig,
    pub file_change_rx: Option<tokio::sync::broadcast::Receiver<kanban_persistence::ChangeEvent>>,
    pub file_watcher: Option<kanban_persistence::FileWatcher>,
    pub save_worker_handle: Option<tokio::task::JoinHandle<()>>,
    pub save_completion_rx: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
}

impl PersistenceState {
    pub fn new(
        save_file: Option<String>,
        app_config: AppConfig,
        save_completion_rx: Option<tokio::sync::mpsc::UnboundedReceiver<()>>,
    ) -> Self {
        Self {
            save_file,
            app_config,
            file_change_rx: None,
            file_watcher: None,
            save_worker_handle: None,
            save_completion_rx,
        }
    }
}
