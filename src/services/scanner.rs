use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;
use thiserror::Error;
use tracing::{debug, error, warn};

use crate::model::file_entry::{FileEntry, FileEntryKind};
use crate::model::scan_node::ScanNode;

#[derive(Debug, Clone)]
pub struct DirSizeResult {
    pub path: PathBuf,
    pub size: u64,
    pub partial: bool,
    pub issue: Option<ScanFailureKind>,
    pub tree: ScanNode,
}
#[derive(Debug, Clone)]
pub struct DirSizeError {
    pub path: PathBuf,
    pub kind: ScanFailureKind,
    pub message: String,
}
#[derive(Debug, Clone)]
pub enum DirSizeEvent {
    Done(DirSizeResult),
    Failed(DirSizeError),
}

#[derive(Debug, Clone)]
struct DirSizeSummary {
    size: u64,
    partial: bool,
    issue: Option<ScanFailureKind>,
    tree: ScanNode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanFailureKind {
    AccessDenied,
    ReadDir,
    SymlinkSkipped,
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("scan cancelled")]
    Cancelled,
}

impl ScanError {
    pub fn failure_kind(&self) -> ScanFailureKind {
        match self {
            Self::ReadDir { source, .. } if source.kind() == io::ErrorKind::PermissionDenied => {
                ScanFailureKind::AccessDenied
            }
            Self::ReadDir { .. } => ScanFailureKind::ReadDir,
            Self::Cancelled => ScanFailureKind::ReadDir,
        }
    }
}

fn merge_issue(current: Option<ScanFailureKind>, next: ScanFailureKind) -> Option<ScanFailureKind> {
    match (current, next) {
        (Some(ScanFailureKind::AccessDenied), _) => Some(ScanFailureKind::AccessDenied),
        (Some(ScanFailureKind::ReadDir), ScanFailureKind::SymlinkSkipped) => {
            Some(ScanFailureKind::ReadDir)
        }
        (Some(existing), _) => Some(existing),
        (None, kind) => Some(kind),
    }
}

pub fn list_directory(path: PathBuf) -> Result<Vec<FileEntry>, ScanError> {
    let mut entries = Vec::new();

    let read_dir = fs::read_dir(&path).map_err(|source| ScanError::ReadDir {
        path: path.clone(),
        source,
    })?;

    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(err) => {
                warn!(error = %err, "skipping unreadable directory entry");
                continue;
            }
        };

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) => {
                warn!(
                    path = %path.display(),
                    error = %err,
                    "skipping entry with unreadable metadata"
                );
                continue;
            }
        };

        let file_type = metadata.file_type();
        let kind = if file_type.is_symlink() {
            FileEntryKind::Symlink
        } else if file_type.is_dir() {
            FileEntryKind::Directory
        } else {
            FileEntryKind::File
        };

        if matches!(kind, FileEntryKind::Symlink) {
            debug!(
                path = %path.display(),
                "found symlinked entry during directory listing"
            );
        }

        entries.push(FileEntry {
            path,
            name,
            kind,
            size: if matches!(kind, FileEntryKind::Directory | FileEntryKind::Symlink) {
                0
            } else {
                metadata.len()
            },
            size_pending: matches!(kind, FileEntryKind::Directory),
            size_partial: false,
        });
    }

    Ok(entries)
}

pub fn compute_directory_sizes_async(
    entries: Vec<FileEntry>,
    tx: Sender<DirSizeEvent>,
    cancel_flag: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        for entry in entries {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            if !matches!(entry.kind, FileEntryKind::Directory) {
                continue;
            }

            match dir_size(&entry.path, &cancel_flag) {
                Ok(summary) => {
                    if cancel_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    if tx.send(DirSizeEvent::Done(DirSizeResult {
                        path: entry.path,
                        size: summary.size,
                        partial: summary.partial,
                        issue: summary.issue,
                        tree: summary.tree,
                    })).is_err() {
                        tracing::warn!("receiver dropped, stopping directory size worker");
                        break;
                    }
                }
                Err(ScanError::Cancelled) => {
                    break;
                }
                Err(err) => {
                    tracing::error!(
                        path = %entry.path.display(),
                        error = %err,
                        "failed to compute directory size"
                    );

                    if tx.send(DirSizeEvent::Failed(DirSizeError {
                        path: entry.path,
                        kind: err.failure_kind(),
                        message: err.to_string(),
                    })).is_err() {
                        tracing::warn!("receiver dropped, stopping directory size worker");
                        break;
                    }
                }
            }
        }
    });
}

fn dir_size(path: &Path, cancel_flag: &AtomicBool) -> Result<DirSizeSummary, ScanError> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(ScanError::Cancelled);
    }

    let mut total = 0;
    let mut partial = false;
    let mut issue = None;
    let mut node = ScanNode::new_directory(path.to_path_buf());

    let read_dir = fs::read_dir(path).map_err(|source| ScanError::ReadDir {
        path: path.to_path_buf(),
        source,
    })?;

    for entry_result in read_dir {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        let entry = match entry_result {
            Ok(entry) => entry,
            Err(err) => {
                warn!(error = %err, "skipping unreadable nested entry");
                partial = true;
                issue = merge_issue(issue, ScanFailureKind::ReadDir);
                continue;
            }
        };

        let path = entry.path();

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) => {
                warn!(
                    path = %path.display(),
                    error = %err,
                    "skipping nested entry with unreadable metadata"
                );
                partial = true;
                issue = merge_issue(
                    issue,
                    if err.kind() == io::ErrorKind::PermissionDenied {
                        ScanFailureKind::AccessDenied
                    } else {
                        ScanFailureKind::ReadDir
                    },
                );
                continue;
            }
        };

        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            debug!(
                path = %path.display(),
                "skipping symlinked entry during recursive scan"
            );
            partial = true;
            issue = merge_issue(issue, ScanFailureKind::SymlinkSkipped);
        } else if file_type.is_file() {
            total += metadata.len();
            node.children.push(ScanNode::new_file(
                path.clone(),
                entry.file_name().to_string_lossy().to_string(),
                metadata.len(),
            ));
        } else if file_type.is_dir() {
            match dir_size(&path, cancel_flag) {
                Ok(summary) => {
                    total += summary.size;
                    partial |= summary.partial;
                    node.children.push(summary.tree);
                    if let Some(summary_issue) = summary.issue {
                        issue = merge_issue(issue, summary_issue);
                    }
                }
                Err(ScanError::Cancelled) => {
                    return Err(ScanError::Cancelled);
                }
                Err(err) => {
                    error!(
                        path = %path.display(),
                        error = %err,
                        "failed to scan subdirectory"
                    );
                    let mut failed_child = ScanNode::new_directory_with_name(
                        path.clone(),
                        entry.file_name().to_string_lossy().to_string(),
                    );
                    failed_child.partial = true;
                    partial = true;
                    issue = merge_issue(issue, err.failure_kind());
                    node.children.push(failed_child);
                }
            }
        }
    }

    node.size = total;
    node.partial = partial;
    node.pending = false;

    Ok(DirSizeSummary {
        size: total,
        partial,
        issue,
        tree: node,
    })
}
