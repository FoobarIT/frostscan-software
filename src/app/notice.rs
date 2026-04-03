use crate::i18n::TextKey;
use crate::services::scanner::{ScanError, ScanFailureKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoticeLevel {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeSource {
    Navigation,
    Scan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppNotice {
    pub level: NoticeLevel,
    pub text_key: TextKey,
    pub source: NoticeSource,
    pub sticky: bool,
}

impl AppNotice {
    pub fn should_replace(self, current: Self) -> bool {
        if self.level != current.level {
            return self.level > current.level;
        }

        if self.source == current.source {
            return true;
        }

        if self.sticky != current.sticky {
            return current.sticky && !self.sticky;
        }

        notice_source_priority(self.source) >= notice_source_priority(current.source)
    }

    pub fn from_open_directory_error(err: &ScanError) -> Self {
        let text_key = match err {
            ScanError::ReadDir { source, .. }
                if source.kind() == std::io::ErrorKind::PermissionDenied =>
            {
                TextKey::OpeningFolderAccessDenied
            }
            ScanError::ReadDir { .. } | ScanError::Cancelled => TextKey::OpeningFolderFailed,
        };

        Self {
            level: NoticeLevel::Error,
            text_key,
            source: NoticeSource::Navigation,
            sticky: true,
        }
    }

    pub fn from_scan_failure(kind: ScanFailureKind) -> Self {
        let text_key = match kind {
            ScanFailureKind::AccessDenied => TextKey::PartialScanWarningAccessDenied,
            ScanFailureKind::SymlinkSkipped => TextKey::PartialScanWarningSymlinkSkipped,
            ScanFailureKind::ReadDir => TextKey::PartialScanWarning,
        };

        Self {
            level: NoticeLevel::Warning,
            text_key,
            source: NoticeSource::Scan,
            sticky: false,
        }
    }
}

fn notice_source_priority(source: NoticeSource) -> u8 {
    match source {
        NoticeSource::Navigation => 3,
        NoticeSource::Scan => 1,
    }
}
