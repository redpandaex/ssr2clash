use ssr2clash_lib::{
    desktop::{
        autostart::{registration_command, registration_matches, set_enabled, AutostartBackend},
        lifecycle::{close_action, should_restore, start_hidden, CloseAction},
    },
    model::app::AppError,
};
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn login_launch_stays_hidden_but_manual_launch_opens_the_window() {
    assert!(start_hidden(["ssr2clash.exe", "--autostart"]));
    assert!(!start_hidden(["ssr2clash.exe"]));
}

#[test]
fn closing_hides_only_when_enabled_and_explicit_quit_stops() {
    assert_eq!(close_action(true, false), CloseAction::Hide);
    assert_eq!(close_action(false, false), CloseAction::Quit);
    assert_eq!(close_action(true, true), CloseAction::Quit);
}

#[test]
fn restore_requires_saved_intent_and_a_valid_initialized_profile() {
    assert!(should_restore(true, true));
    assert!(!should_restore(false, true));
    assert!(!should_restore(true, false));
}

struct FakeAutostart {
    enabled: AtomicBool,
    fail: bool,
}
impl AutostartBackend for FakeAutostart {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(self.enabled.load(Ordering::SeqCst))
    }
    fn enable(&self) -> Result<(), AppError> {
        if self.fail {
            return Err(AppError::new("AutostartWrite", "注册自启失败"));
        }
        self.enabled.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn disable(&self) -> Result<(), AppError> {
        self.enabled.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn autostart_setting_reports_actual_backend_state_and_registration_failure() {
    let backend = FakeAutostart {
        enabled: AtomicBool::new(false),
        fail: true,
    };
    assert_eq!(
        set_enabled(&backend, true).unwrap_err().code,
        "AutostartWrite"
    );
    assert!(!backend.is_enabled().unwrap());
    let backend = FakeAutostart {
        enabled: AtomicBool::new(false),
        fail: false,
    };
    assert!(set_enabled(&backend, true).unwrap());
    assert!(!set_enabled(&backend, false).unwrap());
}

#[test]
fn registration_is_quoted_and_belongs_to_the_current_executable() {
    let exe = std::path::Path::new(r"D:\便携 程序\ssr2clash.exe");
    let command = registration_command(exe);
    assert_eq!(command, "\"D:\\便携 程序\\ssr2clash.exe\" --autostart");
    assert!(registration_matches(&command, exe));
    assert!(!registration_matches(
        &command,
        std::path::Path::new(r"D:\moved\ssr2clash.exe")
    ));
    assert!(!registration_matches(
        r"D:\便携 程序\ssr2clash.exe --autostart",
        exe
    ));
}
