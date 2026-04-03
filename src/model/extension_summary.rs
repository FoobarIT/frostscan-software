use std::collections::BTreeMap;

use super::file_entry::{FileEntry, FileEntryKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSummary {
    pub extension: Option<String>,
    pub total_size: u64,
    pub file_count: usize,
}

pub fn aggregate_extensions(entries: &[FileEntry]) -> Vec<ExtensionSummary> {
    let mut by_extension: BTreeMap<Option<String>, ExtensionSummary> = BTreeMap::new();

    for entry in entries {
        if !matches!(entry.kind, FileEntryKind::File) {
            continue;
        }

        let extension = entry
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        let summary = by_extension
            .entry(extension.clone())
            .or_insert_with(|| ExtensionSummary {
                extension,
                total_size: 0,
                file_count: 0,
            });

        summary.total_size += entry.size;
        summary.file_count += 1;
    }

    let mut summaries: Vec<_> = by_extension.into_values().collect();
    summaries.sort_by(|left, right| {
        right
            .total_size
            .cmp(&left.total_size)
            .then_with(|| right.file_count.cmp(&left.file_count))
            .then_with(|| left.extension.cmp(&right.extension))
    });
    summaries
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::aggregate_extensions;
    use crate::model::file_entry::{FileEntry, FileEntryKind};

    fn file(path: &str, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            name: path.to_string(),
            kind: FileEntryKind::File,
            size,
            size_pending: false,
            size_partial: false,
        }
    }

    #[test]
    fn aggregates_sizes_and_counts_case_insensitively() {
        let summaries = aggregate_extensions(&[
            file("main.RS", 10),
            file("lib.rs", 5),
            file("notes.md", 7),
        ]);

        assert_eq!(summaries[0].extension.as_deref(), Some("rs"));
        assert_eq!(summaries[0].total_size, 15);
        assert_eq!(summaries[0].file_count, 2);
        assert_eq!(summaries[1].extension.as_deref(), Some("md"));
    }

    #[test]
    fn groups_files_without_extension() {
        let summaries = aggregate_extensions(&[file("license", 3), file("readme", 7)]);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].extension, None);
        assert_eq!(summaries[0].total_size, 10);
        assert_eq!(summaries[0].file_count, 2);
    }
}
