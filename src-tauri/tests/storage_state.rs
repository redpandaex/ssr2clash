use ssr2clash_lib::{
    model::app::{OutputMode, ProfileConfig, RunMode, SavedState},
    storage::{detect_data_paths, DataPaths, FileStore, Storage},
};

fn cached_state() -> SavedState {
    let mut state = SavedState::fresh();
    state.revision = 3;
    state.settings.profile.source_url = "https://example.invalid/subscribe".into();
    let output = ssr2clash_lib::conversion::compile_subscription(
        include_str!("fixtures/subconverter/input.txt"),
        &ProfileConfig::default(),
        &ssr2clash_lib::preset::load_builtin_preset().unwrap(),
        3,
    )
    .unwrap()
    .output;
    state.last_good = Some(output.clone());
    state.frozen = Some(output);
    state.mode = OutputMode::Snapshot;
    state
}

#[tokio::test]
async fn portable_data_stays_with_the_executable_not_the_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    let exe_dir = dir.path().join("便携版 with spaces");
    std::fs::create_dir(&exe_dir).unwrap();
    let exe = exe_dir.join("ssr2clash.exe");
    std::fs::write(&exe, []).unwrap();
    std::fs::write(exe_dir.join("portable.flag"), []).unwrap();
    let app_data = dir.path().join("app-data");
    let paths = detect_data_paths(&exe, &app_data).unwrap();
    assert_eq!(paths.run_mode, RunMode::Portable);
    FileStore::new(paths)
        .save(&SavedState::fresh())
        .await
        .unwrap();
    assert!(exe_dir.join("data/state.json").is_file());
    assert!(!app_data.exists());
}

#[tokio::test]
async fn installed_data_uses_user_data_directory() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("ssr2clash.exe");
    std::fs::write(&exe, []).unwrap();
    let app_data = dir.path().join("user/app-data");
    let paths = detect_data_paths(&exe, &app_data).unwrap();
    assert_eq!(paths.run_mode, RunMode::Installed);
    FileStore::new(paths)
        .save(&SavedState::fresh())
        .await
        .unwrap();
    assert!(app_data.join("state.json").is_file());
    assert!(!dir.path().join("data").exists());
}

#[tokio::test]
async fn atomic_overwrite_and_reopen_preserve_link_and_exact_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let paths = DataPaths {
        run_mode: RunMode::Portable,
        root: dir.path().join("中文 data"),
    };
    let store = FileStore::new(paths.clone());
    assert!(store.load().await.unwrap().is_none());
    let mut state = cached_state();
    let token = state.subscription_token.clone();
    let snapshot = state.frozen.as_ref().unwrap().yaml.clone();
    store.save(&state).await.unwrap();
    state.settings.port = 26555;
    state.service_enabled = true;
    store.save(&state).await.unwrap();
    let loaded = FileStore::new(paths).load().await.unwrap().unwrap();
    assert_eq!(loaded, state);
    assert_eq!(loaded.subscription_token, token);
    assert_eq!(loaded.frozen.unwrap().yaml.as_bytes(), snapshot.as_bytes());
    assert_ne!(SavedState::fresh().subscription_token, token);
}

#[tokio::test]
async fn unknown_schema_or_corrupt_json_is_not_replaced_with_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileStore::new(DataPaths {
        run_mode: RunMode::Installed,
        root: dir.path().to_owned(),
    });
    let path = dir.path().join("state.json");
    for (raw, code) in [
        ("{\"schemaVersion\":99}", "UnsupportedStateVersion"),
        ("broken-json", "StateCorrupted"),
    ] {
        std::fs::write(&path, raw).unwrap();
        assert_eq!(store.load().await.unwrap_err().code, code);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    }
}

#[tokio::test]
async fn unwritable_portable_data_does_not_fall_back_to_appdata() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("ssr2clash.exe");
    std::fs::write(&exe, []).unwrap();
    std::fs::write(dir.path().join("portable.flag"), []).unwrap();
    std::fs::write(dir.path().join("data"), "not a directory").unwrap();
    let app_data = dir.path().join("app-data");
    let store = FileStore::new(detect_data_paths(&exe, &app_data).unwrap());
    assert_eq!(
        store.save(&SavedState::fresh()).await.unwrap_err().code,
        "StorageWrite"
    );
    assert!(!app_data.exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("data")).unwrap(),
        "not a directory"
    );
}

#[tokio::test]
async fn invalid_frozen_state_cannot_overwrite_saved_state() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileStore::new(DataPaths {
        run_mode: RunMode::Portable,
        root: dir.path().to_owned(),
    });
    let state = cached_state();
    store.save(&state).await.unwrap();
    let mut invalid = state.clone();
    invalid.frozen = None;
    assert_eq!(store.save(&invalid).await.unwrap_err().code, "InvalidState");
    assert_eq!(store.load().await.unwrap().unwrap(), state);
}

#[cfg(windows)]
#[tokio::test]
async fn windows_file_lock_keeps_old_snapshot_and_cleans_temporary_file() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let store = FileStore::new(DataPaths {
        run_mode: RunMode::Portable,
        root: dir.path().to_owned(),
    });
    let state = cached_state();
    store.save(&state).await.unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(dir.path().join("state.json"))
        .unwrap();
    let mut changed = state.clone();
    changed.settings.port = 26666;
    assert_eq!(store.save(&changed).await.unwrap_err().code, "StorageWrite");
    drop(lock);
    assert_eq!(store.load().await.unwrap().unwrap(), state);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
