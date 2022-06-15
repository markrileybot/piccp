use std::fs::{File, metadata};
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, stderr, Stderr, stdin, stdout, Write};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crossterm::event::EventStream;
use futures::StreamExt;
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
use crate::codec::{Decoder, Encoder};
use crate::frame::Frame;
use crate::log::Log;
use crate::message::Message;
use crate::transport::{Input, InputFactory, Transport};

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

struct StdinInput {
    next_offset: usize
}
impl Input for StdinInput {
    fn read_segment(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        if offset != self.next_offset {
            return Result::Err(Error::from(ErrorKind::InvalidData));
        }
        let stdin = stdin();
        let mut lock = stdin.lock();
        let result = lock.read(buf);
        self.next_offset += 1;
        return result;
    }
}

struct StdinInputFactory {
}
impl InputFactory for StdinInputFactory {
    type InputType = StdinInput;
    fn create_input(&self) -> Self::InputType {
        return StdinInput {next_offset: 0};
    }
}

struct FileInput {
    file: File,
    size: usize
}
impl FileInput {
    fn new(path: String) -> Self {
        return Self {
            file: File::open(path.clone()).unwrap(),
            size: metadata(path).unwrap().len() as usize
        };
    }
}
impl Input for FileInput {
    fn size(&self) -> Option<usize> {
        return Some(self.size);
    }
    fn read_segment(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        self.file.seek(SeekFrom::Start((offset * buf.len()) as u64)).unwrap();
        return self.file.read(buf);
    }
}

struct FileInputFactory {
    path: String
}
impl InputFactory for FileInputFactory{
    type InputType = FileInput;
    fn create_input(&self) -> Self::InputType {
        return FileInput::new(self.path.clone());
    }
}


async fn next_message(ui_state: UiState, encoder: &Encoder, rx: &mut UnboundedReceiver<Message>) -> UiState {
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
                        block_text: encoder.encode(&frame),
                        segment_offset: frame.get_segment_offset(),
                        segment_count: frame.get_segment_count(),
                        message: format!("Sending {}b segment #{}", frame.get_data().len(), frame.get_segment_offset()),
                        ..ui_state
                    }
                } else if frame.is_cts() {
                    UiState {
                        block_text: encoder.encode(&frame),
                        message: format!("Clear to send segment #{}", frame.get_segment_offset()),
                        ..ui_state
                    }
                } else {
                    UiState {
                        block_text: encoder.encode(&frame),
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

        let segment_num = terminal_state.segment_offset + 1;
        let mut progress = Gauge::default()
            .block(Block::default().title("progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black).add_modifier(Modifier::ITALIC));
        if terminal_state.segment_count >= segment_num {
            progress = progress
                .label(Span::from(format!("Segment {}/{}", segment_num, terminal_state.segment_count)))
                .ratio(segment_num as f64 / terminal_state.segment_count as f64);
        } else {
            progress = progress
                .label(Span::from(format!("Segment {}", segment_num)))
                .ratio(0f64);
        }
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
    let transport = if args.input_file.is_empty() {
        Transport::new(tx.clone(), log.clone(), StdinInputFactory{}, args.fragment_size).await
    } else {
        Transport::new(tx.clone(), log.clone(), FileInputFactory{path: args.input_file.clone()}, args.fragment_size).await
    };
    let encoder = Encoder::new(args.scale_width as u32, args.scale_height as u32, !args.hide_quiet_zone);
    let _camera = Camera::new(transport.clone(), Decoder::new(log));

    if !args.is_sender() {
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
            res0 = next_message(current_ui_state.clone(), &encoder, &mut rx) => res0,
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
