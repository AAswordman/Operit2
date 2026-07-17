use std::fmt::{Display, Formatter};
use std::io::Write;

#[derive(Clone, Debug)]
pub(crate) struct CliError {
    message: String,
    location: Option<&'static std::panic::Location<'static>>,
    backtrace: Option<String>,
}

impl CliError {
    /// Creates an internal CLI error with source location and backtrace details.
    #[track_caller]
    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: Some(std::panic::Location::caller()),
            backtrace: Some(std::backtrace::Backtrace::force_capture().to_string()),
        }
    }

    /// Creates an expected user-facing CLI error without internal diagnostics.
    pub(crate) fn user(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            backtrace: None,
        }
    }
}

impl Display for CliError {
    /// Formats the CLI error and its available Rust diagnostics.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)?;
        if let Some(location) = self.location {
            write!(
                formatter,
                "\nRust error location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )?;
        }
        if let Some(backtrace) = &self.backtrace {
            write!(formatter, "\nRust backtrace:\n{backtrace}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CliError {}

/// Installs the terminal-aware panic hook for the CLI process.
pub(crate) fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        restore_terminal_for_panic();
        render_panic_screen(panic_info);
    }));
}

/// Restores the user's terminal before rendering a crash screen.
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

/// Renders the terminal crash screen after raw and alternate-screen modes are released.
fn render_panic_screen(panic_info: &std::panic::PanicHookInfo<'_>) {
    let panic_message = if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    };
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0),
        crossterm::style::SetForegroundColor(crossterm::style::Color::Red),
        crossterm::style::Print("Operit2 has stopped\n\n"),
        crossterm::style::ResetColor,
        crossterm::style::Print("A Rust panic prevented this session from continuing.\n\n"),
        crossterm::style::Print(format!("Panic: {panic_message}\n")),
    );
    if let Some(location) = panic_info.location() {
        let _ = crossterm::execute!(
            stdout,
            crossterm::style::Print(format!(
                "Location: {}:{}:{}\n",
                location.file(),
                location.line(),
                location.column()
            )),
        );
    }
    let _ = crossterm::execute!(
        stdout,
        crossterm::style::Print(format!(
            "\nRust backtrace:\n{}\n\nPress Enter to exit.",
            std::backtrace::Backtrace::force_capture()
        )),
    );
    let _ = stdout.flush();
}

/// Waits for the user to dismiss the rendered terminal crash screen.
pub(crate) fn wait_for_panic_screen() {
    let mut stdin = std::io::stdin();
    let _ = std::io::Read::read(&mut stdin, &mut [0]);
}
