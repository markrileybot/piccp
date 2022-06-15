use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::{Frame, Log};
use crate::message::Message;

pub trait Input: Send {
    fn size(&self) -> Option<usize> {return None;}
    fn read_segment(&mut self, offset: usize, buf: &mut [u8]) -> std::io::Result<usize>;
}

pub trait InputFactory: Send {
    type InputType: Input;
    fn create_input(&self) -> Self::InputType;
}

#[derive(Clone)]
pub struct Transport {
    sender_tx: UnboundedSender<Message>,
    receiver_tx: UnboundedSender<Message>
}

impl Transport {
    pub async fn new<I: 'static>(frame_handler: UnboundedSender<Message>,
                        log: Log,
                        input_factory: I,
                        fragment_size: u16) -> Self
        where I: InputFactory
    {
        let sender_tx = Self::start_sender(frame_handler.clone(), input_factory, fragment_size).await;
        let receiver_tx = Self::start_receiver(frame_handler, log,sender_tx.clone()).await;
        return Self {
            sender_tx,
            receiver_tx
        }
    }

    pub fn receive(&self) {
        self.receiver_tx.send(Message::ReceiveNextFrame).unwrap();
    }

    pub fn receive_frame(&self, frame: Frame) {
        self.receiver_tx.send(Message::ReceiveFrame(frame)).unwrap();
    }

    pub fn send(&self) {
        self.sender_tx.send(Message::SendFrame(0)).unwrap();
    }

    async fn start_receiver(frame_handler: UnboundedSender<Message>,
                            log: Log,
                            frame_sender: UnboundedSender<Message>) -> UnboundedSender<Message> {
        let (tx, mut rx) = unbounded_channel();
        let receiver_tx = tx.clone();
        tokio::spawn(async move {
            let mut expected_offset: usize = 0;
            loop {
                match rx.recv().await.expect("No messages") {
                    Message::ReceiveNextFrame => {
                        frame_handler.send(Message::WriteData(Frame::new_cts(expected_offset))).unwrap();
                    }
                    Message::ReceiveFrame(data) => {
                        if data.is_cts() {
                            expected_offset = data.get_segment_offset();
                            frame_sender.send(Message::SendFrame(expected_offset)).unwrap();
                        } else if data.is_done() {
                            frame_handler.send(Message::WriteData(Frame::new_done())).unwrap();
                            frame_handler.send(Message::Donzo).unwrap();
                            receiver_tx.send(Message::Donzo).unwrap();
                            frame_sender.send(Message::Donzo).unwrap();
                        } else if data.is_segment() {
                            let offset = data.get_segment_offset();
                            if expected_offset == offset {
                                frame_handler.send(Message::AppendToOutput(data)).unwrap();
                                receiver_tx.send(Message::ReceiveNextFrame).unwrap();
                                expected_offset += 1;
                            } else {
                                log.log(format!("Unexpected segment {}", offset));
                            }
                        }
                    }
                    Message::Donzo => {
                        return;
                    }
                    _ => {
                    }
                }
            }
        });
        return tx;
    }

    async fn start_sender<I: 'static>(frame_handler: UnboundedSender<Message>,
                             input_factory: I,
                             fragment_size: u16) -> UnboundedSender<Message>
        where I: InputFactory
    {
        let (tx, mut rx) = unbounded_channel();
        tokio::spawn(async move {
            let mut input = input_factory.create_input();
            let mut buf = vec![0u8; fragment_size as usize];
            let num_segments = match input.size() {
                None => 0,
                Some(s) => s / fragment_size as usize,
            };
            loop {
                match rx.recv().await.expect("No messages") {
                    Message::SendFrame(offset) => {
                        match input.read_segment(offset, &mut buf) {
                            Ok(size) => {
                                frame_handler.send(Message::WriteData(Frame::new_segment(offset, num_segments, &buf[0..size]))).unwrap();
                            }
                            Err(_) => {
                                frame_handler.send(Message::WriteData(Frame::new_done())).unwrap();
                                break;
                            }
                        }
                    }
                    Message::Donzo => {
                        return;
                    }
                    _ => {}
                }
            }
        });
        return tx;
    }
}