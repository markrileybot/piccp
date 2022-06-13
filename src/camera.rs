use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use nokhwa::{CameraFormat, FrameFormat};

use crate::codec::Codec;
use crate::Transport;

pub struct Camera {
    done: Arc<AtomicBool>
}

impl Camera {
    pub async fn new(transport: Transport) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let my_done = done.clone();
        tokio::spawn(async move {
            let mut camera = nokhwa::Camera::new(
                0, // index
                Some(CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)), // format
            ).expect("Failed to get camera");
            camera.open_stream().expect("Failed to open stream");
            let mut codec = Codec::new(transport);
            loop {
                codec.decode(camera.frame().unwrap());
                if my_done.load(Ordering::SeqCst) {
                    return;
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