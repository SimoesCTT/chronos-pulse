//! # Chronos Pulse TUI
//! Real-time phase-space system telemetry terminal dashboard with optional JSONL logging.

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, Borders, Gauge, Paragraph,
    },
    Terminal,
};
use std::{
    env,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
use temporal_engine::{TelemetryFrame, TemporalEngine};

struct CliArgs {
    log_path: Option<PathBuf>,
}

fn parse_args() -> CliArgs {
    let mut log_path = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--log" || arg == "-l" {
            if let Some(path) = args.next() {
                log_path = Some(PathBuf::from(path));
            }
        }
    }
    CliArgs { log_path }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = parse_args();
    let log_file = if let Some(path) = args.log_path {
        Some(File::create(path)?)
    } else {
        None
    };

    let engine = TemporalEngine::new(Duration::from_millis(50), 128);
    let mut rx = engine.spawn();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_ui_loop(&mut terminal, &mut rx, log_file).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run_ui_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    rx: &mut tokio::sync::watch::Receiver<TelemetryFrame>,
    mut log_file: Option<File>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(16);
    let mut last_tick = Instant::now();

    loop {
        let frame_data = rx.borrow().clone();

        // Optional JSONL Logging
        if let Some(file) = log_file.as_mut() {
            if let Ok(json) = serde_json_line(&frame_data) {
                let _ = writeln!(file, "{}", json);
            }
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Length(3), // Gauges (CPU & Mem)
                    Constraint::Length(4), // Per-core Frequencies & Disk I/O
                    Constraint::Min(10),   // Phase-Space Canvas
                    Constraint::Length(3), // Footer
                ])
                .split(f.area());

            // 1. Header
            let status_text = if frame_data.anomaly_detected {
                Span::styled(
                    " [!] PHASE DISRUPTION DETECTED ",
                    Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    " [OK] HARMONIC LOCK NOMINAL ",
                    Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
                )
            };

            let header = Paragraph::new(Line::from(vec![
                Span::styled(
                    " CHRONOS PULSE ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" | "),
                Span::raw(format!("Resonance: {:.1} Hz", frame_data.phase.frequency)),
                Span::raw(" | "),
                Span::raw(format!("Jitter: {:.2} ms", frame_data.jitter_ms)),
                Span::raw(" | "),
                status_text,
            ]))
            .block(Block::default().borders(Borders::ALL).title(" Temporal Telemetry "));
            f.render_widget(header, chunks[0]);

            // 2. Resource Gauges
            let gauge_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let cpu_pct = (frame_data.cpu_ratio * 100.0).clamp(0.0, 100.0) as u16;
            let cpu_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(" CPU Load "))
                .gauge_style(Style::default().fg(Color::Cyan))
                .percent(cpu_pct);
            f.render_widget(cpu_gauge, gauge_chunks[0]);

            let mem_pct = (frame_data.mem_ratio * 100.0).clamp(0.0, 100.0) as u16;
            let mem_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(" Memory Load "))
                .gauge_style(Style::default().fg(Color::Magenta))
                .percent(mem_pct);
            f.render_widget(mem_gauge, gauge_chunks[1]);

            // 3. Per-Core Frequencies & Disk I/O Summary
            let cores_summary = if frame_data.core_frequencies.is_empty() {
                "No per-core frequency telemetry available".to_string()
            } else {
                frame_data
                    .core_frequencies
                    .iter()
                    .enumerate()
                    .map(|(i, mhz)| format!("C{}: {:.0}MHz", i, mhz))
                    .collect::<Vec<_>>()
                    .join(" | ")
            };

            let read_kb = frame_data.disk_read_bytes_sec / 1024.0;
            let write_kb = frame_data.disk_write_bytes_sec / 1024.0;
            let disk_summary = format!("Disk I/O -> Read: {:.1} KB/s | Write: {:.1} KB/s", read_kb, write_kb);

            let sub_info = Paragraph::new(vec![
                Line::from(vec![Span::styled("Cores: ", Style::default().fg(Color::Yellow)), Span::raw(cores_summary)]),
                Line::from(vec![Span::styled("Throughput: ", Style::default().fg(Color::Yellow)), Span::raw(disk_summary)]),
            ])
            .block(Block::default().borders(Borders::ALL).title(" Hardware Subsystems "));
            f.render_widget(sub_info, chunks[2]);

            // 4. Phase-Space Trajectory Canvas
            let points: Vec<(f64, f64)> = frame_data
                .history
                .iter()
                .map(|p| (p.real, p.imag))
                .collect();

            let current_pos = (frame_data.phase.real, frame_data.phase.imag);
            let energy = frame_data.phase.energy_density();

            let canvas = Canvas::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Phase-Space Projection (Energy Density: {:.3}) ", energy)),
                )
                .marker(Marker::Braille)
                .x_bounds([-1.1, 1.1])
                .y_bounds([-1.1, 1.1])
                .paint(move |ctx| {
                    ctx.draw(&CanvasLine {
                        x1: -1.0,
                        y1: 0.0,
                        x2: 1.0,
                        y2: 0.0,
                        color: Color::DarkGray,
                    });
                    ctx.draw(&CanvasLine {
                        x1: 0.0,
                        y1: -1.0,
                        x2: 0.0,
                        y2: 1.0,
                        color: Color::DarkGray,
                    });
                    ctx.draw(&Points {
                        coords: &points,
                        color: Color::Cyan,
                    });
                    ctx.draw(&CanvasLine {
                        x1: 0.0,
                        y1: 0.0,
                        x2: current_pos.0,
                        y2: current_pos.1,
                        color: if frame_data.anomaly_detected {
                            Color::Red
                        } else {
                            Color::Yellow
                        },
                    });
                });
            f.render_widget(canvas, chunks[3]);

            // 5. Footer
            let footer = Paragraph::new(Line::from(vec![
                Span::styled(" q / Esc ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Quit   |   "),
                Span::raw(format!(
                    "Phase Vector: ({:+.3}, {:+.3})",
                    frame_data.phase.real, frame_data.phase.imag
                )),
            ]))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[4]);
        })?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

// Lightweight zero-dependency manual JSON serializer for TelemetryFrame logging
fn serde_json_line(frame: &TelemetryFrame) -> Result<String, String> {
    Ok(format!(
        "{{\"cpu_ratio\":{},\"mem_ratio\":{},\"frequency\":{},\"jitter_ms\":{},\"anomaly\":{},\"real\":{},\"imag\":{}}}",
        frame.cpu_ratio,
        frame.mem_ratio,
        frame.phase.frequency,
        frame.jitter_ms,
        frame.anomaly_detected,
        frame.phase.real,
        frame.phase.imag
    ))
}
