use std::ptr::NonNull;

use esp_idf_svc::hal::gpio::Pin;
use esp_idf_svc::sys::vision::{
    camera_config_t, camera_fb_location_t_CAMERA_FB_IN_PSRAM, camera_fb_t,
    camera_grab_mode_t_CAMERA_GRAB_LATEST, esp_camera_fb_get, esp_camera_fb_return,
    esp_camera_init, framesize_t_FRAMESIZE_QVGA, ledc_channel_t_LEDC_CHANNEL_0,
    ledc_timer_t_LEDC_TIMER_0, pixformat_t_PIXFORMAT_RGB565,
};
use esp_idf_svc::sys::{esp, EspError};

use crate::board::CameraPins;

// At 20 MHz the GC0308 outruns the driver's copy into PSRAM while detection also uses PSRAM.
const XCLK_HZ: i32 = 10_000_000;

pub struct Camera {
    _pins: CameraPins,
}

impl Camera {
    pub fn new(pins: CameraPins) -> Result<Self, EspError> {
        let number = |pin: &dyn PinNumber| pin.number();
        let mut config: camera_config_t = unsafe { core::mem::zeroed() };
        config.pin_pwdn = -1;
        config.pin_reset = -1;
        config.pin_xclk = number(&pins.xclk);
        config.__bindgen_anon_1.pin_sccb_sda = number(&pins.sda);
        config.__bindgen_anon_2.pin_sccb_scl = number(&pins.scl);
        config.pin_d0 = number(&pins.data[0]);
        config.pin_d1 = number(&pins.data[1]);
        config.pin_d2 = number(&pins.data[2]);
        config.pin_d3 = number(&pins.data[3]);
        config.pin_d4 = number(&pins.data[4]);
        config.pin_d5 = number(&pins.data[5]);
        config.pin_d6 = number(&pins.data[6]);
        config.pin_d7 = number(&pins.data[7]);
        config.pin_vsync = number(&pins.vsync);
        config.pin_href = number(&pins.href);
        config.pin_pclk = number(&pins.pclk);
        config.xclk_freq_hz = XCLK_HZ;
        config.ledc_timer = ledc_timer_t_LEDC_TIMER_0;
        config.ledc_channel = ledc_channel_t_LEDC_CHANNEL_0;
        config.pixel_format = pixformat_t_PIXFORMAT_RGB565;
        config.frame_size = framesize_t_FRAMESIZE_QVGA;
        config.fb_count = 2;
        config.fb_location = camera_fb_location_t_CAMERA_FB_IN_PSRAM;
        config.grab_mode = camera_grab_mode_t_CAMERA_GRAB_LATEST;
        esp!(unsafe { esp_camera_init(&config) })?;
        Ok(Self { _pins: pins })
    }

    pub fn capture(&mut self) -> Option<Frame<'_>> {
        NonNull::new(unsafe { esp_camera_fb_get() }).map(|fb| Frame { fb, _camera: self })
    }
}

trait PinNumber {
    fn number(&self) -> i32;
}

impl<P: Pin> PinNumber for P {
    fn number(&self) -> i32 {
        self.pin() as i32
    }
}

pub struct Frame<'a> {
    fb: NonNull<camera_fb_t>,
    _camera: &'a mut Camera,
}

impl Frame<'_> {
    pub fn pixels(&self) -> &[u8] {
        let fb = unsafe { self.fb.as_ref() };
        unsafe { core::slice::from_raw_parts(fb.buf, fb.len) }
    }

    pub fn width(&self) -> u16 {
        unsafe { self.fb.as_ref() }.width as u16
    }

    pub fn height(&self) -> u16 {
        unsafe { self.fb.as_ref() }.height as u16
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        unsafe { esp_camera_fb_return(self.fb.as_ptr()) };
    }
}
