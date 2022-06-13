use bardecoder::Decoder;
use image::{DynamicImage, GrayImage, ImageBuffer, Rgb};
use qrcode::QrCode;

use crate::Frame;
use crate::transport::Transport;

pub struct Codec {
    expected_frame: usize,
    transport: Transport,
    decoder: Decoder<DynamicImage, GrayImage, String>
}

impl Codec {
    pub fn new(transport: Transport) -> Self {
        return Self {
            expected_frame: 0,
            decoder: bardecoder::default_decoder(),
            transport
        }
    }

    pub fn encode(frame: &Frame) -> String {
        let code = QrCode::new(frame).expect("Failed to generate qrcode!");
        return code.render()
            .light_color(' ')
            .dark_color('█')
            .build();
    }

    pub fn decode(&mut self, image: ImageBuffer<Rgb<u8>, Vec<u8>>) {
        let vec = self.decoder.decode(&DynamicImage::ImageRgb8(image.into()));
        for x in vec {
            match x {
                Ok(data) => {
                    let frame = Frame::new(data.into_bytes());
                    if frame.get_sequence() == self.expected_frame {
                        self.expected_frame += 1;
                        self.transport.receive_frame(frame);
                    }
                }
                Err(err) => {
                    println!("{:?}", err)
                }
            }
        }
    }
}