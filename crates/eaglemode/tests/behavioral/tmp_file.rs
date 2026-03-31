use emcore::emTmpFile::{emTmpFile, emTmpFileMaster};
use std::path::PathBuf;

fn test_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("eaglemode_test_tmpfile");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Create an isolated temp directory for master tests to avoid conflicts.
fn master_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eaglemode_test_master_{name}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).ok();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn custom_path_file_deleted_on_drop() {
    let path = test_dir().join("drop_test.txt");
    std::fs::write(&path, b"hello").unwrap();
    assert!(path.exists());

    {
        let _tmp = emTmpFile::from_custom_path(path.clone());
        assert!(path.exists());
    }
    // After drop, file is deleted
    assert!(!path.exists());
}

#[test]
fn custom_path_dir_deleted_on_drop() {
    let dir = test_dir().join("drop_dir_test");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub/file.txt"), b"data").unwrap();
    assert!(dir.exists());

    {
        let _tmp = emTmpFile::from_custom_path(dir.clone());
    }
    assert!(!dir.exists());
}

#[test]
fn get_path() {
    let path = test_dir().join("getpath_test.txt");
    std::fs::write(&path, b"x").unwrap();

    let tmp = emTmpFile::from_custom_path(path.clone());
    assert_eq!(tmp.GetPath(), &path);
    // Explicit discard so we clean up
    drop(tmp);
}

#[test]
fn discard_clears_path() {
    let path = test_dir().join("discard_test.txt");
    std::fs::write(&path, b"x").unwrap();

    let mut tmp = emTmpFile::from_custom_path(path.clone());
    tmp.Discard();

    assert!(tmp.GetPath().as_os_str().is_empty());
    assert!(!path.exists());
}

#[test]
fn empty_tmpfile_drop_is_noop() {
    let _tmp = emTmpFile::new();
    // Should not panic on drop
}

#[test]
fn test_master_singleton_acquires_lock() {
    let dir = master_test_dir("acquire");
    let master = emTmpFileMaster::acquire(&dir);
    assert!(master.is_some(), "first acquisition should succeed");
}

#[test]
fn test_master_registers_and_cleans() {
    let dir = master_test_dir("register");
    let tmp_path = dir.join("test_tmp_file.dat");
    std::fs::write(&tmp_path, b"data").unwrap();
    assert!(tmp_path.exists());

    let mut master = emTmpFileMaster::acquire(&dir).unwrap();
    master.register(&tmp_path);
    assert!(master.is_registered(&tmp_path));

    master.unregister(&tmp_path);
    assert!(!master.is_registered(&tmp_path));
}

#[test]
fn test_master_cleans_on_drop() {
    let dir = master_test_dir("drop");
    let tmp_path = dir.join("cleanup_test.dat");
    std::fs::write(&tmp_path, b"data").unwrap();

    {
        let mut master = emTmpFileMaster::acquire(&dir).unwrap();
        master.register(&tmp_path);
    } // master drops here

    // Registered file should be cleaned up
    assert!(
        !tmp_path.exists(),
        "registered file should be deleted on master drop"
    );
}
