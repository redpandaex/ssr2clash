use crate::model::app::AppError;
use std::path::Path;
use tauri::AppHandle;

pub fn registration_command(exe: &Path) -> String {
    format!("\"{}\" --autostart", exe.display())
}

pub fn registration_matches(command: &str, exe: &Path) -> bool {
    command
        .trim()
        .eq_ignore_ascii_case(&registration_command(exe))
}

pub trait AutostartBackend {
    fn is_enabled(&self) -> Result<bool, AppError>;
    fn enable(&self) -> Result<(), AppError>;
    fn disable(&self) -> Result<(), AppError>;
}

pub fn set_enabled(backend: &impl AutostartBackend, enabled: bool) -> Result<bool, AppError> {
    if enabled {
        backend.enable()?;
    } else {
        backend.disable()?;
    }
    let actual = backend.is_enabled()?;
    if actual != enabled {
        return Err(AppError::new(
            "AutostartMismatch",
            "系统中的自启状态未按预期更新",
        ));
    }
    Ok(actual)
}

pub struct SystemAutostart {
    app: AppHandle,
}
impl SystemAutostart {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use windows_registry::CURRENT_USER;
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const NAME: &str = "SSR2Clash";
    const NOT_FOUND: i32 = 0x80070002u32 as i32;
    fn registry_error() -> AppError {
        AppError::new(
            "AutostartRegistry",
            "无法读写当前用户的自启设置，请检查 Windows 权限",
        )
    }
    fn exe() -> Result<std::path::PathBuf, AppError> {
        std::env::current_exe().map_err(|_| AppError::new("ExecutablePath", "无法确定当前程序路径"))
    }
    fn current_command() -> Result<Option<String>, AppError> {
        match CURRENT_USER.open(RUN).and_then(|key| key.get_string(NAME)) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.code().0 == NOT_FOUND => Ok(None),
            Err(_) => Err(registry_error()),
        }
    }
    fn approved() -> Result<bool, AppError> {
        match CURRENT_USER
            .open(APPROVED)
            .and_then(|key| key.get_value(NAME))
        {
            Ok(bytes) => Ok(matches!(bytes.first(), Some(2 | 6))),
            Err(error) if error.code().0 == NOT_FOUND => Ok(true),
            Err(_) => Err(registry_error()),
        }
    }
    impl AutostartBackend for SystemAutostart {
        fn is_enabled(&self) -> Result<bool, AppError> {
            // The plugin's default Windows query only checks a name exists, not its executable.
            let _ = &self.app;
            let current_exe = exe()?;
            Ok(current_command()?
                .is_some_and(|command| registration_matches(&command, &current_exe))
                && approved()?)
        }
        fn enable(&self) -> Result<(), AppError> {
            let current_exe = exe()?;
            CURRENT_USER
                .create(RUN)
                .and_then(|key| key.set_string(NAME, registration_command(&current_exe)))
                .map_err(|_| registry_error())?;
            // Match Windows Task Manager's enabled format, only for our current-user entry.
            if let Ok(key) = CURRENT_USER.options().write().open(APPROVED) {
                key.set_bytes(
                    NAME,
                    windows_registry::Type::Bytes,
                    &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                )
                .map_err(|_| registry_error())?;
            }
            Ok(())
        }
        fn disable(&self) -> Result<(), AppError> {
            let current_exe = exe()?;
            if current_command()?
                .is_some_and(|command| registration_matches(&command, &current_exe))
            {
                CURRENT_USER
                    .options()
                    .write()
                    .open(RUN)
                    .and_then(|key| key.remove_value(NAME))
                    .map_err(|_| registry_error())?;
                if let Ok(key) = CURRENT_USER.options().write().open(APPROVED) {
                    let _ = key.remove_value(NAME);
                }
            }
            Ok(())
        }
    }
}

#[cfg(not(windows))]
impl AutostartBackend for SystemAutostart {
    fn is_enabled(&self) -> Result<bool, AppError> {
        use tauri_plugin_autostart::ManagerExt;
        self.app
            .autolaunch()
            .is_enabled()
            .map_err(|_| AppError::new("AutostartQuery", "无法读取系统自启状态"))
    }
    fn enable(&self) -> Result<(), AppError> {
        use tauri_plugin_autostart::ManagerExt;
        self.app
            .autolaunch()
            .enable()
            .map_err(|_| AppError::new("AutostartWrite", "注册自启失败"))
    }
    fn disable(&self) -> Result<(), AppError> {
        use tauri_plugin_autostart::ManagerExt;
        self.app
            .autolaunch()
            .disable()
            .map_err(|_| AppError::new("AutostartWrite", "关闭自启失败"))
    }
}
