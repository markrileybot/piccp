use crate::Frame;

#[derive(Debug, Clone)]
pub enum Message {
    SendFrame(usize),
    ReceiveNextFrame,
    ReceiveFrame(Frame),

    WriteData(Frame),
    AppendToOutput(Frame),
    Donzo
}