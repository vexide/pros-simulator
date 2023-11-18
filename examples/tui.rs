use std::{
    ffi::OsString,
    io::{stdout, Result},
    path::Path,
    sync::{Arc, Mutex},
};

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use pros_simulator::{host::lcd::LcdLines, interface::SimulatorEvent, simulate};
use ratatui::{
    prelude::{CrosstermBackend, Stylize, Terminal},
    widgets::Paragraph,
};

#[tokio::main]
async fn main() -> Result<()> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
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
    let robot_code = Path::new(binary_name.as_os_str());

    let lcd_lines = None;

    simulate(robot_code, |event| match event {
        SimulatorEvent::LcdInitialized => {
            lcd_lines = Some(Default::default());
        }
        SimulatorEvent::LcdUpdated(lines) => {
            lcd_lines = Some(lines);
        }
        _ => {}
    })
    .await
    .unwrap();

    loop {
        terminal.draw(|frame| {
            let area = frame.size();
            frame.render_widget(
                Paragraph::new("Hello Ratatui! (press 'q' to quit)")
                    .white()
                    .on_blue(),
                area,
            );
        })?;
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
