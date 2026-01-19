//! Watch folder service for desktop application
//!
//! Monitors a directory for .torrent files and automatically loads them.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rustatio_core::{FakerConfig, TorrentInfo};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, RwLock};

/// Watch folder configuration
#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub enabled: bool,
    pub path: PathBuf,
    pub auto_start: bool,
}

/// Watch folder service for desktop
pub struct WatchService {
    config: WatchConfig,
    app_handle: AppHandle,
    loaded_hashes: Arc<RwLock<HashSet<[u8; 20]>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl WatchService {
    pub fn new(config: WatchConfig, app_handle: AppHandle) -> Self {
        Self {
            config,
            app_handle,
            loaded_hashes: Arc::new(RwLock::new(HashSet::new())),
            shutdown_tx: None,
        }
    }

    /// Start watching the folder
    pub async fn start(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        if !self.config.path.exists() {
            return Err(format!("Watch folder does not exist: {:?}", self.config.path));
        }

        // Scan existing files on startup
        self.scan_directory().await;

        // Start file watcher
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let watch_path = self.config.path.clone();
        let auto_start = self.config.auto_start;
        let app_handle = self.app_handle.clone();
        let loaded_hashes = self.loaded_hashes.clone();

        tokio::spawn(async move {
            if let Err(e) = run_watcher(watch_path, auto_start, app_handle, loaded_hashes, shutdown_rx).await {
                log::error!("Watch service error: {}", e);
            }
        });

        log::info!("Watch folder service started: {:?} (auto_start={})", self.config.path, self.config.auto_start);
        Ok(())
    }

    /// Stop watching
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
            log::info!("Watch folder service stopped");
        }
    }

    /// Scan directory for existing files
    async fn scan_directory(&self) {
        let entries = match std::fs::read_dir(&self.config.path) {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("Failed to scan watch directory: {}", e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if is_torrent_file(&path) {
                if let Err(e) = self.process_torrent_file(&path).await {
                    log::warn!("Failed to process {:?}: {}", path, e);
                }
            }
        }
    }

    /// Process a single torrent file
    async fn process_torrent_file(&self, path: &Path) -> Result<(), String> {
        // Read and parse torrent
        let data = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let torrent = TorrentInfo::from_bytes(&data).map_err(|e| format!("Failed to parse torrent: {}", e))?;

        let info_hash = torrent.info_hash;

        // Check for duplicates
        {
            let hashes = self.loaded_hashes.read().await;
            if hashes.contains(&info_hash) {
                log::debug!("Skipping duplicate torrent: {}", torrent.name);
                return Ok(());
            }
        }

        // Emit event to frontend to create instance with this torrent
        #[derive(Clone, serde::Serialize)]
        struct WatchFolderTorrentEvent {
            torrent: TorrentInfo,
            auto_start: bool,
        }

        let _ = self.app_handle.emit("watch-folder-torrent", WatchFolderTorrentEvent {
            torrent,
            auto_start: self.config.auto_start,
        });

        // Mark as loaded
        self.loaded_hashes.write().await.insert(info_hash);
        log::info!("Loaded torrent from watch folder: {:?}", path.file_name());

        Ok(())
    }
}

/// Check if path is a .torrent file
fn is_torrent_file(path: &Path) -> bool {
    path.is_file() && path.extension().map(|e| e == "torrent").unwrap_or(false)
}

/// Run the file watcher
async fn run_watcher(
    watch_path: PathBuf,
    auto_start: bool,
    app_handle: AppHandle,
    loaded_hashes: Arc<RwLock<HashSet<[u8; 20]>>>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel(100);

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        },
        Config::default(),
    )
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    watcher
        .watch(&watch_path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch directory: {}", e))?;

    log::debug!("File watcher started for {:?}", watch_path);

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                break;
            }
            Some(event) = rx.recv() => {
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for path in event.paths {
                        if is_torrent_file(&path) {
                            // Small delay to ensure file is fully written
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                            // Read and process the torrent
                            match std::fs::read(&path) {
                                Ok(data) => {
                                    match TorrentInfo::from_bytes(&data) {
                                        Ok(torrent) => {
                                            let info_hash = torrent.info_hash;
                                            
                                            // Check if already loaded
                                            let already_loaded = {
                                                let hashes = loaded_hashes.read().await;
                                                hashes.contains(&info_hash)
                                            };

                                            if !already_loaded {
                                                // Emit event to frontend
                                                #[derive(Clone, serde::Serialize)]
                                                struct WatchFolderTorrentEvent {
                                                    torrent: TorrentInfo,
                                                    auto_start: bool,
                                                }

                                                let _ = app_handle.emit("watch-folder-torrent", WatchFolderTorrentEvent {
                                                    torrent,
                                                    auto_start,
                                                });

                                                // Mark as loaded
                                                loaded_hashes.write().await.insert(info_hash);
                                                log::info!("New torrent detected: {:?}", path.file_name());
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!("Failed to parse {:?}: {}", path, e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::warn!("Failed to read {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
