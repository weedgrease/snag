pub mod app;
pub mod dialogs;
pub mod tabs;
pub mod theme;
pub mod utils;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

/// Initializes the terminal (raw mode, alternate screen), runs the TUI event loop, and restores
/// the terminal on exit or panic.
pub async fn run() -> Result<()> {
    let _ = tui_logger::init_logger(log::LevelFilter::Trace);
    tui_logger::set_default_level(log::LevelFilter::Trace);

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new()?;
    let result = app.run(&mut terminal);
    let pending_update = app.pending_update.clone();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Some(info) = pending_update {
        println!("Updating snag to {}...", info.latest_version);
        crate::update::perform_update(&info).await?;
        println!("Restart snag to use the new version.");
        return Ok(());
    }

    result
}
