use std::io::{stderr, Stderr, stdout, Write};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::event::EventStream;
use futures::StreamExt;
use tokio::io::{stdin, Stdin};
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
    widgets::{Block, Borders}
};
use tui::layout::Alignment;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Text};
use tui::widgets::{Gauge, Paragraph};

use crate::args::Args;
use crate::camera::Camera;
use crate::codec::Codec;
use crate::frame::Frame;
use crate::log::Log;
use crate::message::Message;
use crate::transport::{InputFactory, Transport};

mod args;
mod transport;
mod frame;
mod message;
mod camera;
mod codec;
mod log;


#[derive(Debug, Clone)]
struct UiState {
    block_text: String,
    segment_offset: usize,
    segment_count: usize,
    message: String,
    done: bool,
}

impl UiState {
    fn new() -> Self {
        return Self {
            block_text: "".to_string(),
            message: "".to_string(),
            segment_offset: 0,
            segment_count: 0,
            done: false
        }
    }
}

struct StdinInputFactory {
}
impl InputFactory for StdinInputFactory {
    type Output = Stdin;
    fn create_input(&self) -> Self::Output {
        return stdin();
    }
}

async fn next_message(ui_state: UiState, rx: &mut UnboundedReceiver<Message>) -> UiState {
    return if let Some(message) = rx.recv().await {
        match message {
            Message::Log(log) => {
                UiState {
                    message: log,
                    ..ui_state
                }
            },
            Message::WriteData(frame) => {
                if frame.is_segment() {
                    UiState {
                        done: frame.is_done(),
                        block_text: Codec::encode(&frame),
                        segment_offset: frame.get_segment_offset(),
                        segment_count: frame.get_segment_count(),
                        message: format!("Sending segment #{}", frame.get_segment_offset()),
                        ..ui_state
                    }
                } else if frame.is_cts() {
                    UiState {
                        done: frame.is_done(),
                        block_text: Codec::encode(&frame),
                        message: format!("Clear to send segment #{}", frame.get_segment_offset()),
                        ..ui_state
                    }
                } else {
                    UiState {
                        done: frame.is_done(),
                        block_text: Codec::encode(&frame),
                        message: "Done".to_string(),
                        ..ui_state
                    }
                }
            },
            Message::AppendToOutput(frame) => {
                let mut lock = stdout().lock();
                let data = frame.get_data();
                lock.write(data).unwrap();
                UiState {
                    segment_offset: frame.get_segment_offset(),
                    segment_count: frame.get_segment_count(),
                    message: format!("Append {} bytes", data.len()),
                    ..ui_state
                }
            },
            _ => {
                ui_state
            }
        }
    } else {
        ui_state
    }
}

async fn next_input(ui_state: UiState, event_stream: &mut EventStream) -> UiState {
    let mut result = ui_state;
    if let Some(Ok(event)) = event_stream.next().await {
        if event == Event::Key(KeyCode::Esc.into()) {
            result = UiState {done: true, ..result};
        }
    }
    result
}

fn update_ui(terminal: &mut Terminal<CrosstermBackend<Stderr>>, terminal_state: UiState) {
    terminal.draw(|f| {
        let size = f.size();

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(5), Constraint::Length(3)].as_ref())
            .split(size);

        let bot_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(main_chunks[1]);

        let graph = Paragraph::new(Text::from(terminal_state.block_text))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .block(Block::default().title("piccp").borders(Borders::ALL));
        f.render_widget(graph, main_chunks[0]);

        let progress = Gauge::default()
            .block(Block::default().title("progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black).add_modifier(Modifier::ITALIC))
            .label(Span::from(
                format!("Segment {}/{}", terminal_state.segment_offset + 1, terminal_state.segment_count)
            ))
            .ratio(if terminal_state.segment_count > 0 { (terminal_state.segment_offset + 1) as f64 / terminal_state.segment_count as f64 } else { 0f64 });
        f.render_widget(progress, bot_chunks[0]);

        let graph = Paragraph::new(Text::from(terminal_state.message))
            .block(Block::default().title("log").borders(Borders::ALL));
        f.render_widget(graph, bot_chunks[1]);
    }).unwrap();
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (tx, mut rx) = unbounded_channel();
    let log = Log::new(tx.clone());
    let transport = Transport::new(tx.clone(), log.clone(), StdinInputFactory{}, args.fragment_size).await;
    let _camera = Camera::new(transport.clone(), log);

    if !args.send {
        transport.receive();
    }

    enable_raw_mode().unwrap();
    let mut stdout = stderr();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut event_stream = EventStream::new();
    let mut ui_state = UiState::new();

    update_ui(&mut terminal, ui_state.clone());
    loop {
        let current_ui_state = ui_state.clone();
        ui_state = select! {
            res0 = next_message(current_ui_state.clone(), &mut rx) => res0,
            res1 = next_input(current_ui_state, &mut event_stream) => res1,
        };

        update_ui(&mut terminal, ui_state.clone());

        if ui_state.done {
            break;
        }
    }

    // restore terminal
    disable_raw_mode().unwrap();
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    terminal.show_cursor().unwrap();
}
