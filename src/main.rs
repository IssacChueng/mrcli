mod app;
mod config;
mod db;
mod drafts;
mod model;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use app::App;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

fn main() -> Result<()> {
    setup_terminal()?;
    let result = run_app();
    restore_terminal()?;
    result
}

fn run_app() -> Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new();

    while !app.should_quit {
        let size = terminal.size()?;
        let result_viewport = ui::result_viewport(Rect::new(0, 0, size.width, size.height));
        app.set_result_viewport(result_viewport.0, result_viewport.1);
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key);
            }
        }

        app.on_tick();
    }

    Ok(())
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    Ok(())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
