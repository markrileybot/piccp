use image::{DynamicImage, ImageBuffer, Rgb};
use qrcode::{EcLevel, QrCode};
use quircs::Quirc;

use crate::Frame;
use crate::log::Log;

pub struct Encoder {
    width: u32,
    height: u32,
    quiet_zone: bool
}

impl Encoder {
    pub fn new(width: u32, height: u32, quiet_zone: bool) -> Self {
        return Self {
            width,
            height,
            quiet_zone
        }
    }
    pub fn encode(&self, frame: &Frame) -> String {
        let code = QrCode::with_error_correction_level(frame, EcLevel::L)
            .expect("Failed to generate qrcode!");
        return code.render()
            .quiet_zone(self.quiet_zone)
            .module_dimensions(self.width, self.height)
            .light_color('█')
            .dark_color(' ')
            .build();
    }
}

pub struct Decoder {
    log: Log,
    decoder: Quirc
}
impl Decoder {
    pub fn new(log: Log) -> Self {
        return Self {
            decoder: Quirc::default(),
            log,
        }
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
                            result.push(Frame::new(data.payload));
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