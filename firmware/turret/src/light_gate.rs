use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use esp_idf_svc::hal::gpio::{AnyInputPin, Input, InterruptType, Pin, PinDriver, Pull};
use esp_idf_svc::sys::{
    esp, esp_timer_get_time, gpio_install_isr_service, gpio_intr_enable, gpio_isr_handler_add,
    EspError, ESP_ERR_INVALID_STATE,
};

static BLOCKED: AtomicBool = AtomicBool::new(false);
static BLOCKED_AT_US: AtomicU32 = AtomicU32::new(0);
static BLOCKED_FOR_US: AtomicU32 = AtomicU32::new(0);

pub struct LightGate {
    _pin: PinDriver<'static, Input>,
}

impl LightGate {
    // esp-idf-hal's subscribe() disables the GPIO interrupt after every edge; the gate needs
    // both edges of a dart that passes in about 2 ms, so the handler is registered directly.
    pub fn new(pin: AnyInputPin<'static>) -> Result<Self, EspError> {
        let gpio = i32::from(pin.pin());
        let mut pin = PinDriver::input(pin, Pull::Up)?;
        pin.set_interrupt_type(InterruptType::AnyEdge)?;
        match unsafe { gpio_install_isr_service(0) } {
            ESP_ERR_INVALID_STATE => {}
            code => esp!(code)?,
        }
        esp!(unsafe { gpio_isr_handler_add(gpio, Some(on_edge), core::ptr::null_mut()) })?;
        esp!(unsafe { gpio_intr_enable(gpio) })?;
        Ok(Self { _pin: pin })
    }

    pub fn reset(&self) {
        BLOCKED.store(false, Ordering::SeqCst);
        BLOCKED_FOR_US.store(0, Ordering::SeqCst);
    }

    pub fn take_blocked_us(&self) -> Option<u32> {
        match BLOCKED_FOR_US.swap(0, Ordering::SeqCst) {
            0 => None,
            us => Some(us),
        }
    }
}

unsafe extern "C" fn on_edge(_: *mut c_void) {
    let now_us = unsafe { esp_timer_get_time() } as u32;
    if BLOCKED.swap(true, Ordering::SeqCst) {
        let blocked_at = BLOCKED_AT_US.load(Ordering::SeqCst);
        BLOCKED_FOR_US.store(now_us.wrapping_sub(blocked_at).max(1), Ordering::SeqCst);
        BLOCKED.store(false, Ordering::SeqCst);
    } else {
        BLOCKED_AT_US.store(now_us, Ordering::SeqCst);
    }
}
