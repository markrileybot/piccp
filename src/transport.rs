use tokio::io::AsyncRead;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::{Frame, Log, StreamExt};
use crate::message::Message;

pub trait InputFactory: Send {
    type Output: AsyncRead + Unpin + Send;
    fn create_input(&self) -> Self::Output;
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
            let mut reader = FramedRead::new(input_factory.create_input(), BytesCodec::new());

            loop {
                let next = reader.next().await.expect("Failed to get bytes");
                let data = next.expect("Error reading bytes");
                let mut chunks = data.chunks(fragment_size as usize);
                let num_chunks = chunks.len();
                loop {
                    match rx.recv().await.expect("No messages") {
                        Message::SendFrame(offset) => {
                            match chunks.nth(offset) {
                                None => {
                                    frame_handler.send(Message::WriteData(Frame::new_done())).unwrap();
                                }
                                Some(data) => {
                                    frame_handler.send(Message::WriteData(Frame::new_segment(offset, num_chunks, data))).unwrap();
                                }
                            }
                        }
                        Message::Donzo => {
                            return;
                        }
                        _ => {}
                    }
                }
            }
        });
        return tx;
    }
}