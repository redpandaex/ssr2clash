use crate::model::app::{AppError, RunMode};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DataPaths {
    pub run_mode: RunMode,
    pub root: PathBuf,
}

pub fn detect_data_paths(exe: &Path, app_data: &Path) -> Result<DataPaths, AppError> {
    let directory = exe
        .parent()
        .filter(|_| exe.is_absolute())
        .ok_or_else(|| AppError::new("ExecutablePath", "无法确定程序所在目录"))?;
    let marker = directory.join("portable.flag");
    match std::fs::metadata(marker) {
        Ok(metadata) if metadata.is_file() => Ok(DataPaths {
            run_mode: RunMode::Portable,
            root: directory.join("data"),
        }),
        Ok(_) => Err(AppError::new("PortableMarker", "portable.flag 必须是文件")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(DataPaths {
            run_mode: RunMode::Installed,
            root: app_data.to_owned(),
        }),
        Err(_) => Err(AppError::new(
            "PortableMarker",
            "无法检查便携标记，请检查程序目录权限",
        )),
    }
}
