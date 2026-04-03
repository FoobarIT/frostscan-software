use std::fmt;
use std::path::PathBuf;

use sysinfo::Disks;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemDisk {
    pub label: String,
    pub mount_point: PathBuf,
}

impl fmt::Display for SystemDisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

pub fn get_system_disk() -> Vec<SystemDisk> {
    let disks = Disks::new_with_refreshed_list();

    let mut out: Vec<SystemDisk> = disks
        .list()
        .iter()
        .map(|disk| {
            let mount_point = disk.mount_point().to_path_buf();
            let name = disk.name().to_string_lossy().to_string();

            let label = if name.trim().is_empty() {
                mount_point.display().to_string()
            } else {
                format!("{} ({})", name, mount_point.display())
            };

            SystemDisk {label, mount_point}
        })
        .collect();
    out.sort_by(|a, b | a.label.cmp(&b.label));
    out
}
