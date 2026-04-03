use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::unbounded;
use iced::Task;

use crate::app::message::Message;
use crate::app::notice::{AppNotice, NoticeSource};
use crate::app::state::{CachedDirectory, FrostScanApp, SortBy, Tab};
use crate::model::file_entry::{FileEntry, FileEntryKind};
use crate::model::scan_node::ScanNode;
use crate::services::scanner::{ScanFailureKind, compute_directory_sizes_async, list_directory};

impl FrostScanApp {
    fn reset_scan_progress(&mut self) {
        self.scan_progress_done = 0;
        self.scan_progress_total = 0;
    }

    fn clear_navigation_notice(&mut self) {
        if self
            .notice
            .is_some_and(|notice| notice.source == NoticeSource::Navigation)
        {
            self.notice = None;
        }
    }

    fn push_notice(&mut self, notice: AppNotice) {
        let should_replace = match self.notice {
            Some(current) => notice.should_replace(current),
            None => true,
        };

        if should_replace {
            self.notice = Some(notice);
        }
    }

    fn cancel_directory_scan(&mut self) {
        if let Some(cancel_flag) = self.dir_size_cancel.take() {
            cancel_flag.store(true, Ordering::Relaxed);
        }

        self.dir_size_rx = None;
        self.reset_scan_progress();
    }

    fn store_current_directory_in_cache(&mut self) {
        if let (Some(path), Some(tree)) = (self.current_path.clone(), self.scan_tree.clone()) {
            self.directory_cache.insert(
                path,
                CachedDirectory {
                    entries: self.entries.clone(),
                    tree,
                },
            );
        }
    }

    fn finish_directory_scan(&mut self) {
        self.dir_size_rx = None;
        self.dir_size_cancel = None;
        self.reset_scan_progress();
        self.store_current_directory_in_cache();
    }

    fn open_directory(&mut self, path: PathBuf, destination_tab: Tab, force_refresh: bool) {
        if !force_refresh
            && let Some(cached) = self.directory_cache.get(&path).cloned()
        {
            self.cancel_directory_scan();
            self.clear_navigation_notice();
            self.entries = cached.entries;
            self.scan_tree = Some(cached.tree);
            self.current_path = Some(path);
            self.active_tab = destination_tab;
            return;
        }

        self.cancel_directory_scan();

        let entries = match list_directory(path.clone()) {
            Ok(entries) => {
                self.clear_navigation_notice();
                entries
            }
            Err(err) => {
                tracing::error!(
                    path = %path.display(),
                    error = %err,
                    "failed to open directory"
                );

                self.entries.clear();
                self.scan_tree = None;
                self.current_path = Some(path.clone());
                self.push_notice(AppNotice::from_open_directory_error(&err));
                self.active_tab = destination_tab;
                return;
            }
        };

        let directories: Vec<FileEntry> = entries
            .iter()
            .filter(|e| matches!(e.kind, FileEntryKind::Directory))
            .cloned()
            .collect();
        let directories_total = directories.len();

        let (tx, rx) = unbounded();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        compute_directory_sizes_async(directories, tx, Arc::clone(&cancel_flag));

        self.entries = entries;
        self.current_path = Some(path);
        self.scan_tree = self
            .current_path
            .clone()
            .map(|current_path| ScanNode::from_directory_entries(current_path, &self.entries));
        self.dir_size_rx = Some(rx);
        self.dir_size_cancel = Some(cancel_flag);
        self.scan_progress_done = 0;
        self.scan_progress_total = directories_total;
        self.active_tab = destination_tab;

        if directories_total == 0 {
            self.finish_directory_scan();
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
            }

            Message::SelectDisk(disk) => {
                self.selected_disk = Some(disk);
            }

            Message::OpenSelectedDisk => {
                if let Some(disk) = &self.selected_disk {
                    self.open_directory(disk.mount_point.clone(), Tab::Explorer, false);
                }
            }

            Message::OpenEntry(path) => {
                if self
                    .entries
                    .iter()
                    .any(|entry| entry.path == path && matches!(entry.kind, FileEntryKind::Directory))
                {
                    self.open_directory(path, Tab::Explorer, false);
                }
            }

            Message::OpenEntryInTab(path, tab) => {
                if self
                    .entries
                    .iter()
                    .any(|entry| entry.path == path && matches!(entry.kind, FileEntryKind::Directory))
                {
                    self.open_directory(path, tab, false);
                }
            }

            Message::GoUp => {
                if let Some(current) = &self.current_path
                    && let Some(parent) = current.parent()
                {
                    self.open_directory(parent.to_path_buf(), self.active_tab, false);
                }
            }

            Message::Refresh => {
                if let Some(current) = &self.current_path {
                    self.open_directory(current.clone(), self.active_tab, true);
                }
            }

            Message::SortByName => {
                if self.sort_by == SortBy::Name {
                    self.sort_desc = !self.sort_desc;
                } else {
                    self.sort_by = SortBy::Name;
                    self.sort_desc = false;
                }
            }

            Message::SortBySize => {
                if self.sort_by == SortBy::Size {
                    self.sort_desc = !self.sort_desc;
                } else {
                    self.sort_by = SortBy::Size;
                    self.sort_desc = true;
                }
            }

            Message::Tick => {
                if let Some(rx) = self.dir_size_rx.as_ref().cloned() {
                    let mut processed = 0;
                    const MAX_UPDATES_PER_TICK: usize = 20;

                    loop {
                        match rx.try_recv() {
                            Ok(event) => {
                                match event {
                                    crate::services::scanner::DirSizeEvent::Done(result) => {
                                        if let Some(entry) =
                                            self.entries.iter_mut().find(|e| e.path == result.path)
                                        {
                                            entry.size = result.size;
                                            entry.size_pending = false;
                                            entry.size_partial = result.partial;
                                        }
                                        if let Some(tree) = self.scan_tree.as_mut() {
                                            tree.update_child(&result.path, result.tree);
                                        }
                                        self.scan_progress_done += 1;

                                        if result.partial {
                                            self.push_notice(AppNotice::from_scan_failure(
                                                result.issue.unwrap_or(ScanFailureKind::ReadDir),
                                            ));
                                        }
                                    }
                                    crate::services::scanner::DirSizeEvent::Failed(err) => {
                                        if let Some(entry) =
                                            self.entries.iter_mut().find(|e| e.path == err.path)
                                        {
                                            entry.size = 0;
                                            entry.size_pending = false;
                                            entry.size_partial = true;
                                        }
                                        if let Some(tree) = self.scan_tree.as_mut() {
                                            tree.mark_child_failed(&err.path);
                                        }
                                        self.scan_progress_done += 1;

                                        self.push_notice(AppNotice::from_scan_failure(err.kind));

                                        tracing::warn!(
                                            path = %err.path.display(),
                                            kind = ?err.kind,
                                            error = %err.message,
                                            "directory size computation failed"
                                        );
                                    }
                                }

                                processed += 1;
                                if self.scan_progress_done >= self.scan_progress_total
                                    && self.scan_progress_total > 0
                                {
                                    self.finish_directory_scan();
                                    break;
                                }

                                if processed >= MAX_UPDATES_PER_TICK {
                                    break;
                                }
                            }
                            Err(crossbeam_channel::TryRecvError::Empty) => {
                                break;
                            }
                            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                                self.finish_directory_scan();
                                break;
                            }
                        }
                    }
                }
            }

            Message::SetLocale(locale) => {
                self.locale = locale;
            }
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::app::notice::{AppNotice, NoticeLevel, NoticeSource};
    use crate::app::state::{CachedDirectory, FrostScanApp};
    use crate::i18n::TextKey;
    use crate::model::scan_node::ScanNode;

    #[test]
    fn clears_sticky_navigation_notice_after_successful_navigation() {
        let (mut app, _) = FrostScanApp::new();
        app.notice = Some(AppNotice {
            level: NoticeLevel::Error,
            text_key: TextKey::OpeningFolderFailed,
            source: NoticeSource::Navigation,
            sticky: true,
        });

        app.clear_navigation_notice();

        assert!(app.notice.is_none());
    }

    #[test]
    fn keeps_scan_notice_when_clearing_navigation_notice() {
        let (mut app, _) = FrostScanApp::new();
        app.notice = Some(AppNotice {
            level: NoticeLevel::Warning,
            text_key: TextKey::PartialScanWarning,
            source: NoticeSource::Scan,
            sticky: false,
        });

        app.clear_navigation_notice();

        assert!(app.notice.is_some());
    }

    #[test]
    fn open_directory_from_cache_restores_scan_tree() {
        let (mut app, _) = FrostScanApp::new();
        let path = PathBuf::from("cached");
        let tree = ScanNode::new_directory(path.clone());

        app.directory_cache.insert(
            path.clone(),
            CachedDirectory {
                entries: Vec::new(),
                tree: tree.clone(),
            },
        );

        app.open_directory(path.clone(), crate::app::state::Tab::Vue, false);

        assert_eq!(app.current_path.as_ref(), Some(&path));
        assert_eq!(app.scan_tree.as_ref().map(|node| &node.path), Some(&path));
    }

    #[test]
    fn failed_open_clears_previous_scan_tree() {
        let (mut app, _) = FrostScanApp::new();
        app.scan_tree = Some(ScanNode::new_directory(PathBuf::from("old")));

        app.open_directory(
            PathBuf::from("Z:\\definitely-missing-frostscan-path"),
            crate::app::state::Tab::Explorer,
            true,
        );

        assert!(app.scan_tree.is_none());
    }
}
