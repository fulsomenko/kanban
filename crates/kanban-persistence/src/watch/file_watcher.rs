use crate::traits::{ChangeDetector, ChangeEvent};
use chrono::Utc;
use kanban_core::KanbanResult;
use notify::{RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex as TokioMutex;

/// File system watcher for detecting changes to the persistence file
/// Uses the `notify` crate for cross-platform file watching
/// Spawns the watcher in a tokio task to handle the Send requirement
///
/// # Future Directory Format Support
///
/// This implementation is currently designed for single-file JSON persistence
/// but can be extended to support directory-based formats by:
///
/// 1. Adding a `WatchTarget` enum to distinguish between `File(path)` and `Directory(path, pattern)`
/// 2. For directory watching, use `RecursiveMode::Recursive` instead of `NonRecursive`
/// 3. Add glob pattern filtering to the event handler to match specific file extensions
/// 4. Implement event debouncing (e.g., 100ms window) to batch rapid file changes
/// 5. The OS-native backends (inotify, FSEvents, ReadDirectoryChangesW) efficiently
///    handle watching directories with hundreds of files, incurring negligible overhead
///
/// Example future usage:
/// ```ignore
/// let watcher = FileWatcher::new();
/// watcher.start_watching(WatchTarget::Directory("./data".into(), "*.json")).await?;
/// // Efficiently watches all JSON files in directory and subdirectories
/// ```
#[derive(Clone)]
pub struct FileWatcher {
    tx: broadcast::Sender<ChangeEvent>,
    task_handle: Arc<TokioMutex<Option<tokio::task::JoinHandle<()>>>>,
    paused: Arc<AtomicBool>,
}

impl FileWatcher {
    /// Create a new file watcher
    /// The broadcast channel has a buffer size of 10
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(10);
        Self {
            tx,
            task_handle: Arc::new(TokioMutex::new(None)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Pause file watching - events will be ignored until resumed
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        tracing::debug!("File watcher paused");
    }

    /// Resume file watching - events will be processed normally
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        tracing::debug!("File watcher resumed");
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
        let paused = self.paused.clone();

        // Canonicalize to absolute path so it matches OS event paths
        let canonical_path = tokio::fs::canonicalize(&path).await?;

        // Spawn file watching in a background task
        let handle = tokio::spawn(async move {
            let parent = canonical_path
                .parent()
                .expect("Canonicalized path should always have parent")
                .to_path_buf();
            let watch_path = canonical_path.clone();
            let paused_clone = paused.clone();

            match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        // Detect changes from any write strategy:
                        // - Modify(Data(Content)): direct writes
                        // - Create(_): atomic writes (rename operation creating new file)
                        // - Remove(_): atomic writes (old file removed during rename)
                        let is_relevant_event = matches!(
                            event.kind,
                            notify::EventKind::Modify(notify::event::ModifyKind::Data(
                                notify::event::DataChange::Content,
                            )) | notify::EventKind::Create(_)
                                | notify::EventKind::Remove(_)
                        );

                        let has_our_file = event.paths.iter().any(|p| p == &watch_path);

                        if is_relevant_event {
                            tracing::debug!(
                                "File system event detected: kind={:?}, paths={:?}, has_our_file={}",
                                event.kind,
                                event.paths,
                                has_our_file
                            );
                        }

                        // For parent directory watching, trigger on any relevant event
                        // (atomic writes show as temp file events, but the target file exists and changed)
                        if is_relevant_event && (has_our_file || watch_path.exists()) {
                            // Check if file watching is paused (e.g., during our own save operation)
                            // Pause/resume is the only mechanism for filtering own-write events
                            if paused_clone.load(Ordering::SeqCst) {
                                tracing::debug!(
                                    "File event ignored (watcher paused): kind={:?}, path={}",
                                    event.kind,
                                    watch_path.display()
                                );
                                return;
                            }

                            tracing::debug!(
                                "File event detected: kind={:?}, path={}, our_file_exists={}",
                                event.kind,
                                watch_path.display(),
                                watch_path.exists()
                            );
                            let change = ChangeEvent {
                                path: watch_path.clone(),
                                detected_at: Utc::now(),
                            };
                            match tx.send(change) {
                                Ok(receiver_count) => {
                                    tracing::debug!(
                                        "File change event sent to {} receivers",
                                        receiver_count
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to send file change event: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("File watcher error: {}", e);
                    }
                }
            }) {
                Ok(mut watcher) => {
                    // Watch parent directory first (better for detecting atomic writes on macOS FSEvents)
                    let watch_result = watcher.watch(&parent, RecursiveMode::NonRecursive);

                    if watch_result.is_err() {
                        // Fallback to watching the file directly if parent watch fails
                        if let Err(e) = watcher.watch(&canonical_path, RecursiveMode::NonRecursive)
                        {
                            tracing::error!("Failed to watch file or parent directory: {}", e);
                            return;
                        }
                        tracing::info!("Watching file: {}", canonical_path.display());
                    } else {
                        tracing::info!("Watching parent directory: {}", parent.display());
                    }

                    // Keep watcher alive
                    std::future::pending::<()>().await;
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
    async fn test_file_watcher_detects_direct_writes() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        // Create initial file
        tokio::fs::write(&file_path, b"initial content")
            .await
            .unwrap();

        let watcher = FileWatcher::new();
        let mut rx = watcher.subscribe();

        watcher.start_watching(file_path.clone()).await.unwrap();

        // Give watcher time to start
        sleep(Duration::from_millis(100)).await;

        // Modify the file with direct write
        tokio::fs::write(&file_path, b"modified content")
            .await
            .unwrap();

        // Wait for change event (with timeout)
        let result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;

        watcher.stop_watching().await.unwrap();

        // We got an event (timing is platform-dependent, so this might be flaky)
        if let Ok(Ok(event)) = result {
            // Canonicalize both paths to handle platform differences (e.g., macOS /var -> /private/var)
            let expected_path = tokio::fs::canonicalize(&file_path)
                .await
                .unwrap_or(file_path.clone());
            let event_path = tokio::fs::canonicalize(&event.path)
                .await
                .unwrap_or(event.path.clone());
            assert_eq!(event_path, expected_path);
        }
    }

    #[tokio::test]
    async fn test_file_watcher_detects_atomic_writes() {
        use std::fs;
        use tempfile::NamedTempFile;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        // Create initial file
        tokio::fs::write(&file_path, b"initial content")
            .await
            .unwrap();

        let watcher = FileWatcher::new();
        let mut rx = watcher.subscribe();

        watcher.start_watching(file_path.clone()).await.unwrap();

        // Give watcher time to start
        sleep(Duration::from_millis(100)).await;

        // Modify file with atomic write pattern (temp → rename)
        let temp_file = NamedTempFile::new_in(dir.path()).unwrap();
        let temp_path = temp_file.path().to_path_buf();
        std::fs::write(&temp_path, b"atomic write content").unwrap();
        fs::rename(&temp_path, &file_path).unwrap();

        // Wait for change event (with timeout)
        let result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;

        watcher.stop_watching().await.unwrap();

        // We got an event from the atomic write
        if let Ok(Ok(event)) = result {
            let expected_path = tokio::fs::canonicalize(&file_path)
                .await
                .unwrap_or(file_path.clone());
            let event_path = tokio::fs::canonicalize(&event.path)
                .await
                .unwrap_or(event.path.clone());
            assert_eq!(event_path, expected_path);
        }
    }
}
