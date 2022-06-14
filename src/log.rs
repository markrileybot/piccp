use tokio::sync::mpsc::UnboundedSender;

use crate::Message;

#[derive(Clone)]
pub struct Log {
    tx: UnboundedSender<Message>
}

impl Log {
    pub fn new(log_handler: UnboundedSender<Message>) -> Self {
        return Self {
            tx: log_handler
        }
    }

    pub fn log(&self, message: String) {
        self.tx.send(Message::Log(message)).unwrap();
    }
}