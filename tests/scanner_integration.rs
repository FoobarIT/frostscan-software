use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::unbounded;

use frostscan::model::file_entry::FileEntryKind;
use frostscan::model::scan_node::ScanNodeKind;
use frostscan::services::scanner::{DirSizeEvent, compute_directory_sizes_async, list_directory};

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();
    let path = std::env::temp_dir().join(format!("frostscan-{label}-{suffix}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn list_directory_reports_files_and_directories() {
    let root = unique_temp_dir("list-directory");
    fs::write(root.join("readme.txt"), b"hello").unwrap();
    fs::create_dir(root.join("src")).unwrap();

    let entries = list_directory(root.clone()).unwrap();

    assert_eq!(entries.len(), 2);

    let file = entries.iter().find(|entry| entry.name == "readme.txt").unwrap();
    assert!(matches!(file.kind, FileEntryKind::File));
    assert_eq!(file.size, 5);
    assert!(!file.size_pending);

    let directory = entries.iter().find(|entry| entry.name == "src").unwrap();
    assert!(matches!(directory.kind, FileEntryKind::Directory));
    assert_eq!(directory.size, 0);
    assert!(directory.size_pending);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn async_scan_returns_recursive_size_tree() {
    let root = unique_temp_dir("async-tree");
    let docs = root.join("docs");
    let nested = docs.join("nested");

    fs::create_dir(&docs).unwrap();
    fs::create_dir(&nested).unwrap();
    fs::write(root.join("top.bin"), vec![0_u8; 4]).unwrap();
    fs::write(docs.join("guide.md"), vec![0_u8; 6]).unwrap();
    fs::write(nested.join("deep.dat"), vec![0_u8; 9]).unwrap();

    let entries = list_directory(root.clone()).unwrap();
    let (tx, rx) = unbounded();
    let cancel = Arc::new(AtomicBool::new(false));

    compute_directory_sizes_async(entries, tx, cancel);

    let event = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let DirSizeEvent::Done(result) = event else {
        panic!("expected successful scan event");
    };

    assert_eq!(result.path, docs);
    assert_eq!(result.size, 15);
    assert!(!result.partial);
    assert!(result.issue.is_none());
    assert_eq!(result.tree.kind, ScanNodeKind::Directory);

    let nested_node = result
        .tree
        .children
        .iter()
        .find(|node| node.name == "nested")
        .unwrap();
    assert_eq!(nested_node.size, 9);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancelled_scan_stops_without_emitting_results() {
    let root = unique_temp_dir("cancel-scan");
    let folder = root.join("folder");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("file.bin"), vec![0_u8; 32]).unwrap();

    let entries = list_directory(root.clone()).unwrap();
    let (tx, rx) = unbounded();
    let cancel = Arc::new(AtomicBool::new(true));

    compute_directory_sizes_async(entries, tx, cancel);

    let event = rx.recv_timeout(Duration::from_millis(200));
    assert!(event.is_err());

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_entries_produce_partial_scan_warning() {
    use std::os::unix::fs::symlink;
    use frostscan::services::scanner::ScanFailureKind;

    let root = unique_temp_dir("symlink-scan");
    let target = root.join("target");
    let linked = root.join("linked-dir");

    fs::create_dir(&target).unwrap();
    fs::write(target.join("payload.txt"), vec![0_u8; 7]).unwrap();
    symlink(&target, &linked).unwrap();

    let entries = list_directory(root.clone()).unwrap();
    let (tx, rx) = unbounded();
    let cancel = Arc::new(AtomicBool::new(false));

    compute_directory_sizes_async(entries, tx, cancel);

    let event = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let DirSizeEvent::Done(result) = event else {
        panic!("expected successful scan event");
    };

    assert!(result.partial);
    assert_eq!(result.issue, Some(ScanFailureKind::SymlinkSkipped));

    fs::remove_dir_all(root).unwrap();
}
