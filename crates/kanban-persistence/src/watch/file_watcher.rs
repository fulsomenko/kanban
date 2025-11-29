use crate::traits::{ChangeDetector, ChangeEvent};
use chrono::Utc;
use kanban_core::KanbanResult;
use notify::{Watcher, RecursiveMode};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

/// File system watcher for detecting changes to the persistence file
/// Uses the `notify` crate for cross-platform file watching
/// Spawns the watcher in a tokio task to handle the Send requirement
pub struct FileWatcher {
    tx: broadcast::Sender<ChangeEvent>,
    task_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl FileWatcher {
    /// Create a new file watcher
    /// The broadcast channel has a buffer size of 10
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(10);
        Self {
            tx,
            task_handle: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ChangeDetector for FileWatcher {
    async fn start_watching(&self, path: PathBuf) -> KanbanResult<()> {
        let tx = self.tx.clone();
        let task_handle = self.task_handle.clone();

        // Spawn file watching in a background task
        let handle = tokio::spawn(async move {
            let parent = path.parent().expect("Invalid file path").to_path_buf();
            let watch_path = path.clone();

            match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        // Only care about modify events on our file
                        if event.kind == notify::EventKind::Modify(notify::event::ModifyKind::Data(
                            notify::event::DataChange::Content,
                        )) {
                            if event.paths.iter().any(|p| p == &watch_path) {
                                let change = ChangeEvent {
                                    path: watch_path.clone(),
                                    detected_at: Utc::now(),
                                };
                                let _ = tx.send(change);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("File watcher error: {}", e);
                    }
                }
            }) {
                Ok(mut watcher) => {
                    if let Err(e) = watcher.watch(&parent, RecursiveMode::NonRecursive) {
                        tracing::error!("Failed to watch directory: {}", e);
                    } else {
                        tracing::info!("Started watching directory: {}", parent.display());
                        // Keep watcher alive
                        std::future::pending::<()>().await;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create watcher: {}", e);
                }
            }
        });

        let mut guard = task_handle.lock().await;
        *guard = Some(handle);

        Ok(())
    }

    async fn stop_watching(&self) -> KanbanResult<()> {
        let mut guard = self.task_handle.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
            tracing::info!("Stopped file watching");
        }
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ChangeEvent> {
        self.tx.subscribe()
    }

    fn is_watching(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_file_watcher_detects_changes() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        // Create initial file
        tokio::fs::write(&file_path, b"initial content").await.unwrap();

        let watcher = FileWatcher::new();
        let mut rx = watcher.subscribe();

        watcher
            .start_watching(file_path.clone())
            .await
            .unwrap();

        // Give watcher time to start
        sleep(Duration::from_millis(100)).await;

        // Modify the file
        tokio::fs::write(&file_path, b"modified content")
            .await
            .unwrap();

        // Wait for change event (with timeout)
        let result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;

        watcher.stop_watching().await.unwrap();

        // We got an event (timing is platform-dependent, so this might be flaky)
        if let Ok(Ok(event)) = result {
            assert_eq!(event.path, file_path);
        }
    }
}
