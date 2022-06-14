use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc::unbounded_channel;

use crate::codec::Codec;
use crate::log::Log;
use crate::Transport;

pub struct Camera {
    done: Arc<AtomicBool>
}

impl Camera {
    pub fn new(transport: Transport, log: Log) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let my_done = done.clone();
        let (tx, mut rx) = unbounded_channel();
        std::thread::spawn(move || {
            let mut camera = nokhwa::Camera::new(
                0, // index
                None,
            ).expect("Failed to get camera");
            camera.open_stream().expect("Failed to open stream");
            let mut codec = Codec::new(log.clone());
            loop {
                let image = camera.frame().unwrap();
                let result = codec.decode(image);
                if !result.is_empty() {
                    tx.send(result).unwrap();
                }
                if my_done.load(Ordering::SeqCst) {
                    return;
                }
            }
        });

        tokio::spawn(async move {
            loop {
                let frames = rx.recv().await.unwrap();
                for frame in frames {
                    transport.receive_frame(frame);
                }
            }
        });
        return Self {
            done
        };
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
    }
}