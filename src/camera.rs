use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nokhwa::{CameraFormat, FrameFormat, Resolution};

use tokio::sync::mpsc::unbounded_channel;

use crate::codec::Decoder;
use crate::Transport;

pub struct Camera {
    done: Arc<AtomicBool>
}

impl Camera {
    pub fn new(transport: Transport, mut codec: Decoder) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let my_done = done.clone();
        let (tx, mut rx) = unbounded_channel();
        std::thread::spawn(move || {

            let mut camera = nokhwa::Camera::new(
                0, // index
                None,
            ).expect("Failed to get camera");

            match camera.compatible_list_by_resolution(FrameFormat::MJPEG) {
                Ok(resolutions) => {
                    let mut max_resolution = Resolution::default();
                    let mut max_frame_rate = 0;
                    for (resolution, frame_rate) in resolutions {
                        if let Some(mfr) = frame_rate.iter().max() {
                            if mfr >= &30u32 {
                                if resolution.width() * resolution.height() > max_resolution.width() * max_resolution.height() {
                                    max_resolution = resolution;
                                    max_frame_rate = mfr.clone();
                                }
                            }
                        }
                    }
                    if max_frame_rate > 0 {
                        camera.set_camera_format(CameraFormat::new(max_resolution, FrameFormat::MJPEG, max_frame_rate)).unwrap();
                    }
                }
                Err(_) => {}
            }

            camera.open_stream().expect("Failed to open stream");
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