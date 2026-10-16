#[derive(Debug, PartialEq, Eq)]
pub enum CloseAction {
    Hide,
    Quit,
}

pub fn start_hidden<'a>(args: impl IntoIterator<Item = &'a str>) -> bool {
    args.into_iter().any(|argument| argument == "--autostart")
}

pub fn close_action(close_to_tray: bool, explicit_quit: bool) -> CloseAction {
    if close_to_tray && !explicit_quit {
        CloseAction::Hide
    } else {
        CloseAction::Quit
    }
}

pub fn should_restore(saved_intent: bool, initialized: bool) -> bool {
    saved_intent && initialized
}
