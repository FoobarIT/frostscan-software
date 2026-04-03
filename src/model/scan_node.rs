use std::path::{Path, PathBuf};

use crate::model::file_entry::{FileEntry, FileEntryKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanNodeKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct ScanNode {
    pub path: PathBuf,
    pub name: String,
    pub kind: ScanNodeKind,
    pub size: u64,
    pub partial: bool,
    pub pending: bool,
    pub children: Vec<ScanNode>,
}

impl ScanNode {
    pub fn from_directory_entries(path: PathBuf, entries: &[FileEntry]) -> Self {
        let mut children = Vec::new();

        for entry in entries {
            match entry.kind {
                FileEntryKind::File => children.push(Self {
                    path: entry.path.clone(),
                    name: entry.name.clone(),
                    kind: ScanNodeKind::File,
                    size: entry.size,
                    partial: entry.size_partial,
                    pending: false,
                    children: Vec::new(),
                }),
                FileEntryKind::Directory => children.push(Self {
                    path: entry.path.clone(),
                    name: entry.name.clone(),
                    kind: ScanNodeKind::Directory,
                    size: entry.size,
                    partial: entry.size_partial,
                    pending: entry.size_pending,
                    children: Vec::new(),
                }),
                FileEntryKind::Symlink => {}
            }
        }

        let mut root = Self {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string()),
            path,
            kind: ScanNodeKind::Directory,
            size: 0,
            partial: false,
            pending: false,
            children,
        };
        root.refresh_from_children();
        root
    }

    pub fn new_file(path: PathBuf, name: String, size: u64) -> Self {
        Self {
            path,
            name,
            kind: ScanNodeKind::File,
            size,
            partial: false,
            pending: false,
            children: Vec::new(),
        }
    }

    pub fn new_directory(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        Self {
            path,
            name,
            kind: ScanNodeKind::Directory,
            size: 0,
            partial: false,
            pending: false,
            children: Vec::new(),
        }
    }

    pub fn new_directory_with_name(path: PathBuf, name: String) -> Self {
        Self {
            path,
            name,
            kind: ScanNodeKind::Directory,
            size: 0,
            partial: false,
            pending: false,
            children: Vec::new(),
        }
    }

    pub fn update_child(&mut self, path: &Path, child: ScanNode) -> bool {
        if let Some(existing) = self.children.iter_mut().find(|node| node.path == path) {
            *existing = child;
            self.refresh_from_children();
            true
        } else {
            false
        }
    }

    pub fn mark_child_failed(&mut self, path: &Path) -> bool {
        if let Some(existing) = self.children.iter_mut().find(|node| node.path == path) {
            existing.pending = false;
            existing.partial = true;
            existing.size = 0;
            existing.children.clear();
            self.refresh_from_children();
            true
        } else {
            false
        }
    }

    pub fn pending_count(&self) -> usize {
        if self.children.is_empty() {
            usize::from(self.pending)
        } else {
            self.children.iter().map(Self::pending_count).sum()
        }
    }

    pub fn partial_count(&self) -> usize {
        if self.children.is_empty() {
            usize::from(self.partial)
        } else {
            self.children.iter().map(Self::partial_count).sum()
        }
    }

    pub fn refresh_from_children(&mut self) {
        if self.kind == ScanNodeKind::File {
            return;
        }

        self.size = self.children.iter().map(|child| child.size).sum();
        self.partial = self.children.iter().any(|child| child.partial);
        self.pending = self.children.iter().any(|child| child.pending);
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{ScanNode, ScanNodeKind};
    use crate::model::file_entry::{FileEntry, FileEntryKind};

    fn entry(name: &str, kind: FileEntryKind, size: u64, pending: bool) -> FileEntry {
        FileEntry {
            path: PathBuf::from(name),
            name: name.to_string(),
            kind,
            size,
            size_pending: pending,
            size_partial: false,
        }
    }

    #[test]
    fn builds_root_from_directory_entries() {
        let root = ScanNode::from_directory_entries(
            PathBuf::from("/tmp"),
            &[
                entry("file.txt", FileEntryKind::File, 42, false),
                entry("src", FileEntryKind::Directory, 0, true),
            ],
        );

        assert_eq!(root.kind, ScanNodeKind::Directory);
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.size, 42);
        assert!(root.pending);
    }

    #[test]
    fn updates_child_and_recomputes_size() {
        let mut root = ScanNode::from_directory_entries(
            PathBuf::from("/tmp"),
            &[entry("src", FileEntryKind::Directory, 0, true)],
        );

        let mut child = ScanNode::new_directory(PathBuf::from("src"));
        child.size = 120;

        assert!(root.update_child(Path::new("src"), child));
        assert_eq!(root.size, 120);
        assert!(!root.pending);
    }

    #[test]
    fn aggregate_partial_count_does_not_double_count_parents() {
        let mut child = ScanNode::new_directory(PathBuf::from("src"));
        child.partial = true;

        let mut root = ScanNode::new_directory(PathBuf::from("root"));
        root.children.push(child);
        root.refresh_from_children();

        assert!(root.partial);
        assert_eq!(root.partial_count(), 1);
    }

    #[test]
    fn aggregate_pending_count_does_not_double_count_parents() {
        let mut child = ScanNode::new_directory(PathBuf::from("src"));
        child.pending = true;

        let mut root = ScanNode::new_directory(PathBuf::from("root"));
        root.children.push(child);
        root.refresh_from_children();

        assert!(root.pending);
        assert_eq!(root.pending_count(), 1);
    }
}
