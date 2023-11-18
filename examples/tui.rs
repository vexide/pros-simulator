use std::{
    ffi::OsString,
    io::stdout,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures::{TryStream, TryStreamExt};
use pros_simulator::{
    host::lcd::LcdLines, interface::SimulatorEvent, simulate, stream::start_simulator,
};
use ratatui::{
    prelude::{CrosstermBackend, Stylize, Terminal},
    widgets::Paragraph,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        _ = stdout().execute(LeaveAlternateScreen);
        _ = disable_raw_mode();
        panic_hook(panic_info);
    }));
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    // tracing_subscriber::fmt()
    //     .with_env_filter(
    //         EnvFilter::builder()
    //             .with_default_directive(LevelFilter::INFO.into())
    //             .from_env_lossy(),
    //     )
    //     .init();
    let args = std::env::args_os().collect::<Vec<_>>();
    let binary_name = args.get(1).cloned().unwrap_or_else(|| {
        OsString::from("./example/target/wasm32-unknown-unknown/debug/example.wasm")
    });
    let robot_code = PathBuf::from(binary_name);

    let mut lcd_lines = None::<LcdLines>;

    let mut sim_events = start_simulator(robot_code);

    loop {
        // draw to terminal
        terminal.draw(|frame| {
            let area = frame.size();
            if let Some(lcd_lines) = &lcd_lines {
                frame.render_widget(
                    Paragraph::new(format!("Lcd Display:\n{}", lcd_lines.join("\n"))),
                    area,
                );
            } else {
                frame.render_widget(Paragraph::new("LCD not initialized").red(), area);
            }
        })?;

        // handle simulator events
        if let Some(event) = sim_events.try_next().await? {
            match event {
                SimulatorEvent::LcdUpdated(lines) => {
                    lcd_lines = Some(lines);
                }
                SimulatorEvent::LcdInitialized => lcd_lines = Some(LcdLines::default()),
                _ => {}
            }
        }

        // handle keyboard input
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
