use image::{DynamicImage, ImageBuffer, Rgb};
use qrcode::{EcLevel, QrCode};
use quircs::{Quirc};

use crate::Frame;
use crate::log::Log;

pub struct Codec {
    expected_frame: usize,
    log: Log,
    decoder: Quirc
}

impl Codec {
    pub fn new(log: Log) -> Self {
        return Self {
            expected_frame: 0,
            decoder: Quirc::default(),
            log,
        }
    }

    pub fn encode(frame: &Frame) -> String {
        let code = QrCode::with_error_correction_level(frame, EcLevel::L)
            .expect("Failed to generate qrcode!");
        return code.render()
            .quiet_zone(true)
            .module_dimensions(4, 2)
            .light_color('█')
            .dark_color(' ')
            .build();
    }

    pub fn decode(&mut self, image: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Vec<Frame> {
        let dyn_image = DynamicImage::ImageRgb8(image.into());
        let gray_image = dyn_image.into_luma8();
        let vec = self.decoder.identify(gray_image.width() as usize, gray_image.height() as usize, &gray_image);
        let mut result = Vec::new();
        for x in vec {
            match x {
                Ok(data) => {
                    match data.decode() {
                        Ok(data) => {
                            let frame = Frame::new(data.payload);
                            if frame.get_sequence() == self.expected_frame {
                                self.expected_frame += 1;
                                result.push(frame);
                            } else {
                                self.log.log(format!("Unexpected frame {}", frame.get_sequence()));
                            }
                        }
                        Err(err) => {
                            self.log.log(format!("{:?}", err));
                        }
                    }
                }
                Err(err) => {
                    self.log.log(format!("{:?}", err));
                }
            }
        }
        return result;
    }
}