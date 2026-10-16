use core::ptr::NonNull;

use esp_idf_svc::sys::vision::{
    face_box_t, face_detect, face_detector_new, face_detector_t, face_embed, face_embedder_new,
    face_embedder_t, face_embedding_len, face_encode_jpeg, face_image_t, face_jpeg_free,
    FACE_KEYPOINTS,
};
use esp_idf_svc::sys::{esp, EspError, ESP_ERR_NO_MEM, ESP_FAIL};
use vision_core::rect::Rect;

use crate::camera::Frame;

pub const MAX_FACES: usize = 8;
const KEYPOINTS: usize = FACE_KEYPOINTS as usize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detection {
    pub x0: i16,
    pub y0: i16,
    pub x1: i16,
    pub y1: i16,
    pub keypoints: [i16; KEYPOINTS],
    pub score: f32,
}

impl Detection {
    pub fn rect(&self) -> Rect {
        Rect::new(self.x0, self.y0, self.x1, self.y1)
    }
}

#[derive(Default)]
pub struct Image {
    pixels: Vec<u8>,
    width: u16,
    height: u16,
}

impl Image {
    pub fn copy_from(&mut self, frame: &Frame<'_>) {
        self.pixels.clear();
        self.pixels.extend_from_slice(frame.pixels());
        self.width = frame.width();
        self.height = frame.height();
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn mean_luma(&self) -> u8 {
        let samples = self.pixels.chunks_exact(2).step_by(97);
        let (sum, count) = samples.fold((0u32, 0u32), |(sum, count), pixel| {
            let rgb = u16::from_be_bytes([pixel[0], pixel[1]]);
            let r = u32::from(rgb >> 11) * 255 / 31;
            let g = u32::from((rgb >> 5) & 0x3F) * 255 / 63;
            let b = u32::from(rgb & 0x1F) * 255 / 31;
            (sum + (r * 77 + g * 150 + b * 29) / 256, count + 1)
        });
        (sum / count.max(1)) as u8
    }

    pub fn encode_jpeg(&self, quality: u8) -> Result<Vec<u8>, EspError> {
        let mut data = core::ptr::null_mut();
        let mut len = 0;
        esp!(unsafe { face_encode_jpeg(&self.raw(), quality, &mut data, &mut len) })?;
        let jpeg = unsafe { core::slice::from_raw_parts(data, len) }.to_vec();
        unsafe { face_jpeg_free(data) };
        Ok(jpeg)
    }

    fn raw(&self) -> face_image_t {
        face_image_t {
            data: self.pixels.as_ptr().cast_mut(),
            width: self.width,
            height: self.height,
        }
    }
}

pub struct Detector(NonNull<face_detector_t>);

unsafe impl Send for Detector {}

impl Detector {
    pub fn new() -> Result<Self, EspError> {
        NonNull::new(unsafe { face_detector_new() })
            .map(Self)
            .ok_or_else(EspError::from_infallible::<ESP_ERR_NO_MEM>)
    }

    pub fn detect(&mut self, image: &Image) -> heapless::Vec<Detection, MAX_FACES> {
        let mut raw = [face_box_t {
            box_: [0; 4],
            keypoints: [0; KEYPOINTS],
            score: 0.0,
        }; MAX_FACES];
        let count =
            unsafe { face_detect(self.0.as_ptr(), &image.raw(), raw.as_mut_ptr(), MAX_FACES) };
        raw[..count]
            .iter()
            .map(|face| Detection {
                x0: face.box_[0],
                y0: face.box_[1],
                x1: face.box_[2],
                y1: face.box_[3],
                keypoints: face.keypoints,
                score: face.score,
            })
            .collect()
    }
}

pub struct Embedder {
    raw: NonNull<face_embedder_t>,
    len: usize,
}

unsafe impl Send for Embedder {}

impl Embedder {
    pub fn new() -> Result<Self, EspError> {
        let raw = NonNull::new(unsafe { face_embedder_new() })
            .ok_or_else(EspError::from_infallible::<ESP_ERR_NO_MEM>)?;
        let len = unsafe { face_embedding_len(raw.as_ptr()) };
        Ok(Self { raw, len })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn embed(
        &mut self,
        image: &Image,
        face: &Detection,
        out: &mut [f32],
    ) -> Result<(), EspError> {
        if out.len() != self.len {
            return Err(EspError::from_infallible::<ESP_FAIL>());
        }
        esp!(unsafe {
            face_embed(
                self.raw.as_ptr(),
                &image.raw(),
                face.keypoints.as_ptr(),
                out.as_mut_ptr(),
                out.len(),
            )
        })
    }
}
