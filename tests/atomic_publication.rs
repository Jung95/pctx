use pctx::project::atomic_write;
use std::sync::{Arc, Barrier};

#[test]
fn create_only_race_publishes_one_complete_record_and_preserves_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("receipt.json");
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|n| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let bytes = serde_json::to_vec(&serde_json::json!({
                    "writer": n, "payload": "x".repeat(65536),
                }))
                .unwrap();
                barrier.wait();
                atomic_write(&path, &bytes, false).map(|_| bytes)
            })
        })
        .collect();
    let mut winners = Vec::new();
    for handle in handles {
        match handle.join().unwrap() {
            Ok(bytes) => winners.push(bytes),
            Err(e) => assert_eq!(e.code, "REVISION_CONFLICT"),
        }
    }
    assert_eq!(winners.len(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), winners[0]);
    assert_eq!(
        atomic_write(&path, b"replacement", false).unwrap_err().code,
        "REVISION_CONFLICT"
    );
    assert_eq!(std::fs::read(&path).unwrap(), winners[0]);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn replacement_keeps_old_open_reader_and_publishes_complete_new_record() {
    use std::io::Read;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let old = b"{\"revision\":1}";
    let new = serde_json::to_vec(&serde_json::json!({"revision": 2, "payload": "z".repeat(65536)}))
        .unwrap();
    atomic_write(&path, old, false).unwrap();
    let mut reader = std::fs::File::open(&path).unwrap();
    atomic_write(&path, &new, true).unwrap();
    let mut original = Vec::new();
    reader.read_to_end(&mut original).unwrap();
    assert_eq!(original, old);
    assert_eq!(std::fs::read(&path).unwrap(), new);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_posix_replacement_preserves_non_delete_shared_readers_and_unicode_names() {
    use std::{io::Read, os::windows::fs::OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state-한글-🚀.json");
    let revisions = [
        b"first".as_slice(),
        b"second".as_slice(),
        b"third".as_slice(),
    ];
    let mut readers = Vec::new();
    for revision in revisions {
        atomic_write(&path, revision, !readers.is_empty()).unwrap();
        let reader = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(&path)
            .unwrap();
        readers.push(reader);
    }
    for (mut reader, expected) in readers.into_iter().zip(revisions) {
        let mut contents = Vec::new();
        reader.read_to_end(&mut contents).unwrap();
        assert_eq!(contents, expected);
    }
    assert_eq!(std::fs::read(&path).unwrap(), revisions[2]);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_replacement_failure_preserves_directory_target_and_cleans_staging() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("preserved"), b"original").unwrap();
    assert!(atomic_write(&target, b"new file", true).is_err());
    assert_eq!(
        std::fs::read(target.join("preserved")).unwrap(),
        b"original"
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
