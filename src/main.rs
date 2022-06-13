use std::io::{stderr, stdout, Write};
use std::time::SystemTime;

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::event::{EventStream, KeyModifiers};
use futures::StreamExt;
use tokio::io::{stdin, Stdin};
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::oneshot;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
    widgets::{Block, Borders}
};
use tui::layout::Alignment;
use tui::text::Text;
use tui::widgets::Paragraph;

use crate::args::Args;
use crate::camera::Camera;
use crate::codec::Codec;
use crate::frame::Frame;
use crate::message::Message;
use crate::transport::{InputFactory, Transport};

mod args;
mod transport;
mod frame;
mod message;
mod camera;
mod codec;


#[derive(Clone)]
struct UiState {
    block_text: String,
    size_text: String,
    done: bool,
}

impl UiState {
    fn new() -> Self {
        return Self {
            block_text: "".to_string(),
            size_text: "".to_string(),
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
            Message::WriteData(frame) => {
                UiState {
                    done: frame.is_done(),
                    block_text: Codec::encode(&frame),
                    ..ui_state
                }
            },
            Message::AppendToOutput(frame) => {
                let mut lock = stderr().lock();
                let data = frame.get_data();
                lock.write(data).unwrap();
                UiState {
                    size_text: format!("{}", data.len()),
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (tx, mut rx) = unbounded_channel();
    let transport = Transport::new(tx.clone(), StdinInputFactory{}, args.fragment_size).await;
    let _camera = Camera::new(transport.clone()).await;

    if args.send {
        transport.send();
    } else {
        transport.receive();
    }

    enable_raw_mode().unwrap();
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut event_stream = EventStream::new();
    let mut ui_state = UiState::new();

    loop {
        let current_ui_state = ui_state.clone();
        ui_state = select! {
            res1 = next_input(current_ui_state.clone(), &mut event_stream) => res1,
            res2 = next_message(current_ui_state, &mut rx) => res2,
        };

        let terminal_state = ui_state.clone();
        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
                .split(size);

            let block = Block::default()
                .title("piccp")
                .borders(Borders::ALL);
            let graph = Paragraph::new(Text::from(terminal_state.block_text))
                .alignment(Alignment::Center)
                .block(block);
            f.render_widget(graph, chunks[0]);

            let block = Block::default()
                .title("info")
                .borders(Borders::ALL);
            let graph = Paragraph::new(Text::from(format!("Size: {}", terminal_state.size_text)))
                .alignment(Alignment::Center)
                .block(block);
            f.render_widget(graph, chunks[1]);

        }).unwrap();

        if ui_state.done {
            break;
        }
    }

    // restore terminal
    disable_raw_mode().unwrap();
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    terminal.show_cursor().unwrap();
}
