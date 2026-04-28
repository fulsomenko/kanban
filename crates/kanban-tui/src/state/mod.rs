pub mod snapshot;

use std::sync::Arc;
use tokio::sync::mpsc;

pub use snapshot::TuiSnapshot;
const SAVE_QUEUE_CAPACITY: usize = 100;

/// Coordinates persistence with immediate auto-saving
///
/// # Save Behavior
///
/// Changes are saved immediately after each command execution:
/// - No debounce delay - every change is persisted instantly
/// - No data loss window - state is always current on disk
/// - Simple and predictable behavior
///
/// The immediate save approach works well for kanban boards because:
/// - User actions are discrete and human-paced (not rapid-fire)
/// - Modern SSDs handle frequent writes efficiently (1-5ms per save)
/// - Conflict detection and file watching remain responsive
///
/// # Example
/// ```ignore
/// // Execute command via TuiContext, then queue snapshot
/// ctx.execute_commands_batch(commands)?;
/// ```
pub struct SaveCoordinator {
    save_tx: Option<mpsc::Sender<()>>,
    save_completion_tx: Option<mpsc::UnboundedSender<()>>,
    pending_saves: usize,
    file_watcher: Option<Arc<kanban_persistence::FileWatcher>>,
}

impl SaveCoordinator {
    /// Create a new save coordinator with optional persistence store
    ///
    /// Returns a tuple of:
    /// - SaveCoordinator instance
    /// - Optional receiver for async save processing (snapshots to save)
    /// - Optional receiver for save completion notifications
    ///
    #[allow(clippy::type_complexity)]
    pub fn new(
        has_persistence: bool,
    ) -> (
        Self,
        Option<mpsc::Receiver<()>>,
        Option<mpsc::UnboundedReceiver<()>>,
    ) {
        let (save_tx, save_rx) = if has_persistence {
            let (tx, rx) = mpsc::channel(SAVE_QUEUE_CAPACITY);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let (save_completion_tx, save_completion_rx) = mpsc::unbounded_channel();

        let coordinator = Self {
            save_tx,
            save_completion_tx: Some(save_completion_tx),
            pending_saves: 0,
            file_watcher: None,
        };

        (coordinator, save_rx, Some(save_completion_rx))
    }

    /// Close the save channel to signal the worker to finish processing and exit
    /// Called during graceful shutdown before waiting for the worker to finish
    pub fn close_save_channel(&mut self) {
        self.save_tx = None;
    }

    /// Check if the save channel is available for sending snapshots
    pub fn has_save_channel(&self) -> bool {
        self.save_tx.is_some()
    }

    /// Queue a flush signal for async saving.
    /// Increments pending_saves to track unsaved changes.
    ///
    /// Pauses file watcher before queueing to prevent detecting our own writes as external changes.
    /// The save worker will resume the watcher after the save completes.
    ///
    /// Uses try_send to handle bounded channel capacity (100 slots).
    /// If channel is full, logs warning and skips to prevent blocking UI.
    pub fn queue_flush(&mut self) {
        // Pause file watching before queuing save to prevent detecting our own writes
        if let Some(ref watcher) = self.file_watcher {
            watcher.pause();
            tracing::debug!("File watcher paused before queuing flush");
        }

        if let Some(ref tx) = self.save_tx {
            tracing::debug!(
                "Queueing flush signal (pending: {} -> {})",
                self.pending_saves,
                self.pending_saves + 1
            );
            match tx.try_send(()) {
                Ok(_) => {
                    self.pending_saves += 1;
                    tracing::debug!("Snapshot queued successfully");
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    tracing::warn!(
                        "Save queue is full ({} pending), skipping this save. \
                        This may indicate the disk is slow or the save worker is overloaded.",
                        self.pending_saves
                    );
                    // Resume watcher if we couldn't queue the snapshot
                    if let Some(ref watcher) = self.file_watcher {
                        watcher.resume();
                        tracing::debug!("File watcher resumed (save queue full)");
                    }
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    tracing::error!("Failed to queue save: channel closed");
                    // Resume watcher if channel is closed
                    if let Some(ref watcher) = self.file_watcher {
                        watcher.resume();
                        tracing::debug!("File watcher resumed (channel closed)");
                    }
                }
            }
        } else {
            tracing::debug!("No save channel available - skipping save");
            // Resume watcher if no save channel
            if let Some(ref watcher) = self.file_watcher {
                watcher.resume();
                tracing::debug!("File watcher resumed (no save channel)");
            }
        }
    }

    /// Check if there are pending saves waiting to be written
    pub fn has_pending_saves(&self) -> bool {
        self.pending_saves > 0
    }

    /// Signal that a save has been completed (called by save worker)
    pub fn save_completed(&mut self) {
        if self.pending_saves > 0 {
            self.pending_saves -= 1;
            tracing::debug!(
                "Save completed (pending: {} -> {})",
                self.pending_saves + 1,
                self.pending_saves
            );
        }
    }

    /// Get the save completion sender for the worker to use
    pub fn save_completion_tx(&self) -> Option<&mpsc::UnboundedSender<()>> {
        self.save_completion_tx.as_ref()
    }

    #[doc(hidden)]
    pub fn set_pending_for_test(&mut self, n: usize) {
        self.pending_saves = n;
    }

    /// Set the file watcher for coordinating pause/resume with saves
    /// Called after the file watcher is initialized in App::run()
    pub fn set_file_watcher(&mut self, watcher: Arc<kanban_persistence::FileWatcher>) {
        self.file_watcher = Some(watcher);
        tracing::debug!("File watcher set on SaveCoordinator");
    }

    /// Reset save channels after a backend migration.
    ///
    /// Drops the old save channel (causing the old save worker to exit),
    /// creates new channels. Returns receivers so the caller can spawn a new save worker.
    /// The store itself lives on KanbanContext.
    #[allow(clippy::type_complexity)]
    pub fn reset_save_channels(
        &mut self,
    ) -> (mpsc::Receiver<()>, mpsc::UnboundedReceiver<()>) {
        self.file_watcher = None;
        self.pending_saves = 0;

        let (tx, rx) = mpsc::channel(SAVE_QUEUE_CAPACITY);
        self.save_tx = Some(tx);

        let (completion_tx, completion_rx) = mpsc::unbounded_channel();
        self.save_completion_tx = Some(completion_tx);

        (rx, completion_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_coordinator_creation_no_persistence() {
        let (coordinator, save_rx, _completion_rx) = SaveCoordinator::new(false);
        assert!(!coordinator.has_pending_saves());
        assert!(save_rx.is_none());
    }

    #[test]
    fn test_save_coordinator_creation_with_persistence() {
        let (coordinator, save_rx, _completion_rx) = SaveCoordinator::new(true);
        assert!(!coordinator.has_pending_saves());
        assert!(save_rx.is_some());
        assert!(coordinator.has_save_channel());
    }

    #[test]
    fn test_reset_save_channels() {
        let (mut coordinator, _rx, _crx) = SaveCoordinator::new(true);
        let (_rx, _crx) = coordinator.reset_save_channels();
        assert!(!coordinator.has_pending_saves());
        assert!(coordinator.has_save_channel());
    }
}
