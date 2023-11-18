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
    layout::{Constraint, Direction, Layout},
    prelude::{CrosstermBackend, Stylize, Terminal},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget, TuiLoggerWidget};

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

    let tui_log_layer = tui_logger::tracing_subscriber_layer();
    let logger = Registry::default().with(tui_log_layer);
    tracing::subscriber::set_global_default(logger)?;

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
            let size = frame.size();
            let mut constraints = vec![Constraint::Percentage(50), Constraint::Percentage(30)];
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(size);

            if let Some(lcd_lines) = &lcd_lines {
                frame.render_widget(
                    Paragraph::new(format!("Lcd Display:\n{}", lcd_lines.join("\n"))),
                    layout[0],
                );
            } else {
                frame.render_widget(Paragraph::new("LCD not initialized").red(), layout[0]);
            }

            let tui_w = TuiLoggerWidget::default()
                .block(Block::default().title("Logs").borders(Borders::ALL))
                .output_separator(' ')
                .output_timestamp(Some("%H:%M:%S".to_string()))
                .output_level(Some(TuiLoggerLevelOutput::Long))
                .output_target(false)
                .output_file(false)
                .output_line(false)
                .style_error(Style::default().fg(Color::Red))
                .style_debug(Style::default().fg(Color::Green))
                .style_warn(Style::default().fg(Color::Yellow))
                .style_trace(Style::default().fg(Color::Magenta))
                .style_info(Style::default().fg(Color::Cyan));
            frame.render_widget(tui_w, layout[1]);
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
