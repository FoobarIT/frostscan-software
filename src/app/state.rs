use std::collections::HashMap;
use crossbeam_channel::Receiver;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use iced::time;
use iced::{Subscription, Task, Theme};

use crate::app::message::Message;
use crate::app::notice::AppNotice;
use crate::i18n::{Locale, TextKey, t};
use crate::model::file_entry::FileEntry;
use crate::model::scan_node::ScanNode;
use crate::services::disk::{get_system_disk, SystemDisk};

use crate::services::scanner::DirSizeEvent;

#[derive(Debug, Clone)]
pub struct CachedDirectory {
    pub entries: Vec<FileEntry>,
    pub tree: ScanNode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Accueil,
    Explorer,
    Vue,
    Options,
    Aide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
}

pub struct FrostScanApp {
    pub locale: Locale,

    pub active_tab: Tab,
    pub disks: Vec<SystemDisk>,
    pub selected_disk: Option<SystemDisk>,

    pub current_path: Option<PathBuf>,
    pub entries: Vec<FileEntry>,
    pub scan_tree: Option<ScanNode>,
    pub directory_cache: HashMap<PathBuf, CachedDirectory>,
    pub notice: Option<AppNotice>,

    pub sort_by: SortBy,
    pub sort_desc: bool,

    pub dir_size_rx: Option<Receiver<DirSizeEvent>>,
    pub dir_size_cancel: Option<Arc<AtomicBool>>,
    pub scan_progress_done: usize,
    pub scan_progress_total: usize,
}

impl FrostScanApp {
    pub fn new() -> (Self, Task<Message>) {
        (
            FrostScanApp {
                locale: Locale::default(),
                active_tab: Tab::Accueil,
                disks: get_system_disk(),
                selected_disk: None,
                current_path: None,
                entries: Vec::new(),
                scan_tree: None,
                directory_cache: HashMap::new(),
                notice: None,
                sort_by: SortBy::Name,
                sort_desc: false,
                dir_size_rx: None,
                dir_size_cancel: None,
                scan_progress_done: 0,
                scan_progress_total: 0,
            },
            Task::none(),
        )
    }
    pub fn tr(&self, key: TextKey) -> &'static str {
        t(self.locale, key)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.dir_size_rx.is_some() {
            time::every(Duration::from_millis(50)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn scan_in_progress(&self) -> bool {
        self.scan_progress_total > 0 && self.scan_progress_done < self.scan_progress_total
    }

    pub fn scan_progress_ratio(&self) -> f32 {
        if self.scan_progress_total == 0 {
            0.0
        } else {
            self.scan_progress_done as f32 / self.scan_progress_total as f32
        }
    }
}
