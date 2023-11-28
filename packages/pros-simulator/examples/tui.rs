use std::{ffi::OsString, io::stdout, path::PathBuf, process::exit, task::Poll};

use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures::TryStreamExt;
use indoc::indoc;
use pros_simulator::stream::start_simulator;
use pros_simulator_interface::{LcdLines, SimulatorEvent, LCD_HEIGHT, LCD_WIDTH};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::{CrosstermBackend, Stylize, Terminal},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::oneshot;
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tui_big_text::BigTextBuilder;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

#[tokio::main]
async fn main() {
    // trigger panic hook on error
    app().await.unwrap();
}
async fn app() -> anyhow::Result<()> {
    let mut synchronous_redraws = false;
    let mut input_file = None;

    let mut args = std::env::args_os();
    args.next();
    for arg in args {
        match arg.to_str() {
            Some("--help") => {
                const HELP: &str = indoc! {"
                    tui [OPTIONS] [INPUT]

                    OPTIONS:
                        --help                  Print this help message
                        --synchronous-redraws   Always keep the screen up to date with the simulator
                                                state. Slower, but useful when using breakpoints.
                "};
                println!("{HELP}");
                exit(1);
            }
            Some("--synchronous-redraws") => {
                synchronous_redraws = true;
            }
            _ => {
                if input_file.is_none() {
                    input_file = Some(arg);
                } else {
                    return Err(anyhow::anyhow!("Unknown argument"));
                }
            }
        }
    }

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

    let binary_name = input_file.unwrap_or_else(|| {
        OsString::from("./example/target/wasm32-unknown-unknown/debug/example.wasm")
    });
    let robot_code = PathBuf::from(binary_name);

    let mut lcd_lines = None::<LcdLines>;
    let mut sim_events = start_simulator(robot_code, synchronous_redraws);
    let mut loading_state = Some(0);
    let mut unpause = None::<oneshot::Sender<()>>;

    loop {
        // draw to terminal
        terminal.draw(|frame| {
            let size = frame.size();
            let constraints = vec![Constraint::Percentage(50), Constraint::Percentage(30)];
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(size);

            let mut lcd_block = Block::new().title("LCD Display").borders(Borders::ALL);
            lcd_block = lcd_block.border_style(Style::new().reset());
            if let Some(loading_state) = &mut loading_state {
                lcd_block = lcd_block.white().on_blue();
                frame.render_widget(lcd_block.clone(), layout[0]);
                draw_splash_screen(frame, lcd_block.inner(layout[0]), *loading_state).unwrap();
                *loading_state += 1;
            } else if let Some(lcd_lines) = &lcd_lines {
                lcd_block = lcd_block.on_green();
                let inner_size = lcd_block.inner(layout[0]);
                let inner_block = Block::new().black().on_gray();

                let vertical_padding = (inner_size.height - LCD_HEIGHT as u16) / 2;
                let inner_vertical_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(vertical_padding),
                        Constraint::Length(LCD_HEIGHT as u16),
                        Constraint::Length(vertical_padding),
                    ])
                    .split(inner_size);
                let horizontal_padding = (inner_size.width - LCD_WIDTH as u16) / 2;
                let inner_horizontal_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![
                        Constraint::Length(horizontal_padding),
                        Constraint::Length(LCD_WIDTH as u16),
                        Constraint::Length(horizontal_padding),
                    ])
                    .split(inner_vertical_layout[1]);

                frame.render_widget(lcd_block, layout[0]);
                frame.render_widget(
                    Paragraph::new(lcd_lines.join("\n")).block(inner_block),
                    inner_horizontal_layout[1],
                );
            } else {
                // black screen to emulate display off
                lcd_block = lcd_block.on_black();
                frame.render_widget(lcd_block, layout[0]);
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

        if let Some(unpause) = unpause.take() {
            // After we receve an event, we only resume the simulator after we've redrawn the screen.
            // This prevents the simulator from hitting a breakpoint before the screen is updated.
            unpause.send(()).unwrap();
        }

        // handle simulator events
        if let Poll::Ready(event) = futures::poll!(sim_events.try_next()) {
            let event = event?;
            if let Some(event) = event {
                match event.inner {
                    SimulatorEvent::LcdUpdated(lines) => {
                        lcd_lines = Some(lines);
                    }
                    SimulatorEvent::LcdInitialized => lcd_lines = Some(LcdLines::default()),
                    SimulatorEvent::RobotCodeStarting => {
                        loading_state = None;
                    }
                    SimulatorEvent::RobotCodeFinished => {
                        tracing::info!("Press q to quit.");
                    }
                    _ => {}
                }
                unpause = event.unpause;
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

fn draw_splash_screen(frame: &mut Frame, size: Rect, loading_state: usize) -> anyhow::Result<()> {
    let constraints = vec![Constraint::Min(8), Constraint::Min(1)];
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);
    {
        let size = layout[0];
        const MESSAGE: &str = "pros-rs";
        let width = (MESSAGE.len() * 8) as u16;
        let centered_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((size.width - width) / 2),
                Constraint::Length(width),
                Constraint::Length((size.width - width) / 2),
            ])
            .split(size);
        let splash_screen = BigTextBuilder::default()
            .lines(vec![MESSAGE.into()])
            .build()?;
        frame.render_widget(splash_screen, centered_layout[1]);
    }
    let loading_width = layout[1].width as usize;
    let mut loading_text = " ".repeat(loading_width);
    for i in loading_state..loading_state + 3 {
        let i = i % loading_width;
        loading_text.replace_range(i..i + 1, "#");
    }
    frame.render_widget(Paragraph::new(loading_text), layout[1]);

    Ok(())
}
