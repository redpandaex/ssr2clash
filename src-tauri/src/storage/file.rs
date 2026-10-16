use super::{DataPaths, Storage};
use crate::{
    model::{
        app::{AppError, OutputMode, SavedState},
        clash::ClashConfig,
    },
    preset::validate_config,
};
use async_trait::async_trait;
use std::{io::Write, path::PathBuf};
use tokio::io::AsyncReadExt;

const MAX_STATE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct FileStore {
    root: PathBuf,
}
impl FileStore {
    pub fn new(paths: DataPaths) -> Self {
        Self { root: paths.root }
    }
}

#[async_trait]
impl Storage for FileStore {
    async fn load(&self) -> Result<Option<SavedState>, AppError> {
        let file = match tokio::fs::File::open(self.root.join("state.json")).await {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err(AppError::new(
                    "StorageRead",
                    format!("无法读取配置目录：{}", self.root.display()),
                ))
            }
        };
        let mut bytes = Vec::new();
        file.take(MAX_STATE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| AppError::new("StorageRead", "读取应用数据失败"))?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err(AppError::new(
                "StateCorrupted",
                "应用数据文件超过大小限制，原文件已保留",
            ));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| AppError::new("StateCorrupted", "应用数据编码无效，原文件已保留"))?;
        let document: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|_| AppError::new("StateCorrupted", "应用数据 JSON 无效，原文件已保留"))?;
        if document
            .get("schemaVersion")
            .and_then(|value| value.as_u64())
            != Some(1)
        {
            return Err(AppError::new(
                "UnsupportedStateVersion",
                "应用数据版本不兼容，请使用匹配版本；原文件已保留",
            ));
        }
        let state: SavedState = serde_json::from_value(document)
            .map_err(|_| AppError::new("StateCorrupted", "应用数据结构无效，原文件已保留"))?;
        validate_state(&state)
            .map_err(|_| AppError::new("StateCorrupted", "保存的配置或快照无效，原文件已保留"))?;
        Ok(Some(state))
    }

    async fn save(&self, state: &SavedState) -> Result<(), AppError> {
        validate_state(state)?;
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|_| AppError::new("StorageWrite", "无法序列化应用数据"))?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err(AppError::new(
                "StorageWrite",
                "应用数据超过大小限制，旧数据已保留",
            ));
        }
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let failure = || {
                AppError::new(
                    "StorageWrite",
                    format!(
                        "无法保存到配置目录，请检查权限或文件占用：{}",
                        root.display()
                    ),
                )
            };
            std::fs::create_dir_all(&root).map_err(|_| failure())?;
            let mut temporary = tempfile::NamedTempFile::new_in(&root).map_err(|_| failure())?;
            temporary.write_all(&bytes).map_err(|_| failure())?;
            temporary.as_file().sync_all().map_err(|_| failure())?;
            temporary
                .persist(root.join("state.json"))
                .map_err(|_| failure())?;
            Ok(())
        })
        .await
        .map_err(|_| AppError::new("StorageWrite", "保存任务未能完成，旧数据已保留"))?
    }
}

fn validate_state(state: &SavedState) -> Result<(), AppError> {
    let invalid = || AppError::new("InvalidState", "应用配置或快照状态无效，未覆盖旧数据");
    if state.schema_version != 1
        || state.settings.port == 0
        || !(16..=128).contains(&state.subscription_token.len())
        || !state
            .subscription_token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(invalid());
    }
    if state.mode == OutputMode::Snapshot
        && !state
            .frozen
            .as_ref()
            .is_some_and(|snapshot| snapshot.config_revision == state.revision)
    {
        return Err(invalid());
    }
    for snapshot in [&state.last_good, &state.frozen].into_iter().flatten() {
        if snapshot.config_revision > state.revision
            || chrono::DateTime::parse_from_rfc3339(&snapshot.generated_at).is_err()
        {
            return Err(invalid());
        }
        let config: ClashConfig = serde_saphyr::from_str(&snapshot.yaml).map_err(|_| invalid())?;
        validate_config(&config).map_err(|_| invalid())?;
        if snapshot.node_count != config.proxies.len()
            || snapshot.group_count != config.proxy_groups.len()
        {
            return Err(invalid());
        }
    }
    Ok(())
}
