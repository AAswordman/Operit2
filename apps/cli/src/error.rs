use std::fmt::{Display, Formatter};
use std::io::Write;

#[derive(Clone, Debug)]
pub(crate) struct CliError {
    message: String,
    location: &'static std::panic::Location<'static>,
    backtrace: String,
}

impl CliError {
    #[track_caller]
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: std::panic::Location::caller(),
            backtrace: std::backtrace::Backtrace::force_capture().to_string(),
        }
    }
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)?;
        write!(
            formatter,
            "\nRust error location: {}:{}:{}",
            self.location.file(),
            self.location.line(),
            self.location.column()
        )?;
        write!(formatter, "\nRust backtrace:\n{}", self.backtrace)
    }
}

impl std::error::Error for CliError {}

pub(crate) fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        restore_terminal_for_panic();
        eprintln!("Unhandled panic");
        if let Some(location) = panic_info.location() {
            eprintln!(
                "Rust panic location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("Panic payload: {message}");
        } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("Panic payload: {message}");
        } else {
            eprintln!("Panic payload: <non-string>");
        }
        eprintln!(
            "Rust backtrace:\n{}",
            std::backtrace::Backtrace::force_capture()
        );
    }));
}

fn restore_terminal_for_panic() {
    let _ = crossterm::terminal::disable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
        crossterm::terminal::LeaveAlternateScreen
    );
    let _ = stdout.flush();
}
