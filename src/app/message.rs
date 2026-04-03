use super::state::Tab;
use crate::i18n::Locale;
use crate::services::disk::SystemDisk;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(Tab),
    SelectDisk(SystemDisk),
    OpenSelectedDisk,
    OpenEntry(PathBuf),
    OpenEntryInTab(PathBuf, Tab),
    GoUp,
    Refresh,
    SortByName,
    SortBySize,
    Tick,
    SetLocale(Locale),
}
