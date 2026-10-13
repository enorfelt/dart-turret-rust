use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::ble::gap::{BleAdvFields, BleAdvParams, GapEvent};
use esp_idf_svc::ble::gatt::server::{BleGattRegister, GattsEvent};
use esp_idf_svc::ble::{
    ensure_addr, BleDriver, BleError, BleSecurity, BleUuid, ConnHandle, CONN_HANDLE_NONE,
};
use esp_idf_svc::gatt_services;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::sys;
use turret_core::remote::{self, CommandError, RemoteEvent, StateSnapshot, STATE_LEN};

use crate::turret::Input;

const DEVICE_NAME: &str = "DART Turret";
const SERVICE: BleUuid = BleUuid::uuid128(0x8d6f0001_3b2a_4c1e_9f57_2e1d0c7a5b40);
const COMMAND: BleUuid = BleUuid::uuid128(0x8d6f0002_3b2a_4c1e_9f57_2e1d0c7a5b40);
const STATE: BleUuid = BleUuid::uuid128(0x8d6f0003_3b2a_4c1e_9f57_2e1d0c7a5b40);
const EVENT: BleUuid = BleUuid::uuid128(0x8d6f0004_3b2a_4c1e_9f57_2e1d0c7a5b40);
const STATE_INTERVAL: Duration = Duration::from_millis(50);
const KEY_DIST: u8 = (sys::BLE_SM_PAIR_KEY_DIST_ENC | sys::BLE_SM_PAIR_KEY_DIST_ID) as u8;

gatt_services!(SERVICES {
    primary(SERVICE) {
        chr(COMMAND, Write | WriteAuthen);
        chr(STATE, Read | ReadAuthen | Notify | NotifyIndicateAuthen);
        chr(EVENT, Notify | NotifyIndicateAuthen);
    }
});

static COMMAND_HANDLE: AtomicU16 = AtomicU16::new(0);
static STATE_HANDLE: AtomicU16 = AtomicU16::new(0);
static EVENT_HANDLE: AtomicU16 = AtomicU16::new(0);
static PEER: AtomicU16 = AtomicU16::new(CONN_HANDLE_NONE);
static STATE_SUBSCRIBED: AtomicBool = AtomicBool::new(false);
static EVENT_SUBSCRIBED: AtomicBool = AtomicBool::new(false);
static NEEDS_ADVERTISING: AtomicBool = AtomicBool::new(false);
static LATEST_STATE: Mutex<Option<[u8; STATE_LEN]>> = Mutex::new(None);

extern "C" {
    fn ble_store_config_init();
}

enum Outgoing {
    State(StateSnapshot),
    Event(RemoteEvent),
    ForgetBonds,
}

#[derive(Clone)]
pub struct Remote(Sender<Outgoing>);

impl Remote {
    pub fn state(&self, state: StateSnapshot) {
        let _ = self.0.send(Outgoing::State(state));
    }

    pub fn event(&self, event: RemoteEvent) {
        let _ = self.0.send(Outgoing::Event(event));
    }

    pub fn forget_bonds(&self) {
        let _ = self.0.send(Outgoing::ForgetBonds);
    }
}

pub fn start(modem: Modem<'static>, inputs: Sender<Input>, passkey: u32) -> anyhow::Result<Remote> {
    let mut driver = BleDriver::new_with_services(modem, &SERVICES)?;
    driver.set_security(&BleSecurity {
        io_cap: sys::BLE_HS_IO_DISPLAY_ONLY as u8,
        bonding: true,
        mitm: true,
        secure_connections: true,
        our_key_dist: KEY_DIST,
        their_key_dist: KEY_DIST,
        ..BleSecurity::new()
    })?;
    BleError::from_raw(unsafe { sys::ble_sm_configure_static_passkey(passkey, true) })?;
    unsafe { ble_store_config_init() };

    let commands = inputs.clone();
    driver.gatts_subscribe(move |event| on_gatts(event, &commands));
    driver.gap_subscribe(move |event| {
        on_gap(event, &inputs);
        0
    });
    driver.host_subscribe(|event| {
        if let esp_idf_svc::ble::HostEvent::Sync = event {
            NEEDS_ADVERTISING.store(true, Ordering::Relaxed);
        }
    });
    driver.start()?;

    let (outgoing, received) = mpsc::channel();
    thread::Builder::new()
        .name("ble".into())
        .stack_size(4 * 1024)
        .spawn(move || run(driver, received))?;
    Ok(Remote(outgoing))
}

fn on_gatts(event: GattsEvent<'_>, commands: &Sender<Input>) -> u8 {
    match event {
        GattsEvent::Register(BleGattRegister::Characteristic {
            uuid, val_handle, ..
        }) => {
            for (known, handle) in [
                (COMMAND, &COMMAND_HANDLE),
                (STATE, &STATE_HANDLE),
                (EVENT, &EVENT_HANDLE),
            ] {
                if uuid == known {
                    handle.store(val_handle, Ordering::Relaxed);
                }
            }
            0
        }
        GattsEvent::Write {
            attr_handle, data, ..
        } if attr_handle == COMMAND_HANDLE.load(Ordering::Relaxed) => {
            let mut buffer = [0u8; 32];
            let Ok(len) = data.read(&mut buffer) else {
                return CommandError::Malformed.att_code();
            };
            match remote::decode_command(&buffer[..len]) {
                Ok(command) => {
                    let _ = commands.send(Input::Command(command));
                    0
                }
                Err(error) => error.att_code(),
            }
        }
        GattsEvent::Read {
            attr_handle,
            mut reply,
            ..
        } if attr_handle == STATE_HANDLE.load(Ordering::Relaxed) => {
            let state = *LATEST_STATE.lock().unwrap();
            match state.map(|state| reply.append(&state)) {
                Some(Err(_)) => sys::BLE_ATT_ERR_INSUFFICIENT_RES as u8,
                _ => 0,
            }
        }
        GattsEvent::SubscriptionChanged {
            attr_handle,
            cur_notify,
            ..
        } => {
            if attr_handle == STATE_HANDLE.load(Ordering::Relaxed) {
                STATE_SUBSCRIBED.store(cur_notify, Ordering::Relaxed);
            } else if attr_handle == EVENT_HANDLE.load(Ordering::Relaxed) {
                EVENT_SUBSCRIBED.store(cur_notify, Ordering::Relaxed);
            }
            0
        }
        _ => 0,
    }
}

fn on_gap(event: GapEvent, inputs: &Sender<Input>) {
    match event {
        GapEvent::Connect {
            conn_handle,
            status: Ok(()),
        } => {
            PEER.store(conn_handle, Ordering::Relaxed);
            log::info!("remote connected");
        }
        GapEvent::Connect { status: Err(_), .. } => {
            NEEDS_ADVERTISING.store(true, Ordering::Relaxed);
        }
        GapEvent::Disconnect { reason, .. } => {
            PEER.store(CONN_HANDLE_NONE, Ordering::Relaxed);
            STATE_SUBSCRIBED.store(false, Ordering::Relaxed);
            EVENT_SUBSCRIBED.store(false, Ordering::Relaxed);
            NEEDS_ADVERTISING.store(true, Ordering::Relaxed);
            log::info!("remote disconnected: {reason}");
            let _ = inputs.send(Input::RemoteLost);
        }
        _ => {}
    }
}

fn run<S>(driver: BleDriver<'static, S>, outgoing: Receiver<Outgoing>) {
    let mut pending_state = None;
    let mut last_state_at: Option<Instant> = None;
    loop {
        match outgoing.recv_timeout(STATE_INTERVAL) {
            Ok(Outgoing::State(state)) => {
                let encoded = state.encode();
                *LATEST_STATE.lock().unwrap() = Some(encoded);
                pending_state = Some(encoded);
            }
            Ok(Outgoing::Event(event)) => {
                if let Some(state) = pending_state.take() {
                    notify(&STATE_HANDLE, &STATE_SUBSCRIBED, &state);
                    last_state_at = Some(Instant::now());
                }
                notify(&EVENT_HANDLE, &EVENT_SUBSCRIBED, &event.encode());
            }
            Ok(Outgoing::ForgetBonds) => {
                match BleError::from_raw(unsafe { sys::ble_store_clear() }) {
                    Ok(()) => log::info!("paired devices forgotten"),
                    Err(error) => log::warn!("forgetting paired devices failed: {error}"),
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        let state_due = last_state_at.is_none_or(|at| at.elapsed() >= STATE_INTERVAL);
        if state_due {
            if let Some(state) = pending_state.take() {
                notify(&STATE_HANDLE, &STATE_SUBSCRIBED, &state);
                last_state_at = Some(Instant::now());
            }
        }

        if NEEDS_ADVERTISING.swap(false, Ordering::Relaxed) {
            if let Err(error) = advertise(&driver) {
                log::warn!("advertising failed: {error}");
            }
        }
    }
}

fn notify(handle: &AtomicU16, subscribed: &AtomicBool, payload: &[u8]) {
    let peer: ConnHandle = PEER.load(Ordering::Relaxed);
    if peer == CONN_HANDLE_NONE || !subscribed.load(Ordering::Relaxed) {
        return;
    }
    // esp-idf-svc only wraps indications; NimBLE frees the mbuf on every path of notify_custom.
    let mbuf = unsafe { sys::ble_hs_mbuf_from_flat(payload.as_ptr().cast(), payload.len() as u16) };
    if mbuf.is_null() {
        log::warn!("notify dropped: out of mbufs");
        return;
    }
    let sent = unsafe { sys::ble_gatts_notify_custom(peer, handle.load(Ordering::Relaxed), mbuf) };
    if let Err(error) = BleError::from_raw(sent) {
        log::warn!("notify failed: {error}");
    }
}

fn advertise<S>(driver: &BleDriver<'_, S>) -> Result<(), BleError> {
    ensure_addr(false)?;
    driver.set_device_name(DEVICE_NAME)?;
    driver.adv_set_fields(&BleAdvFields {
        flags: (sys::BLE_HS_ADV_F_DISC_GEN | sys::BLE_HS_ADV_F_BREDR_UNSUP) as u8,
        name: Some(DEVICE_NAME),
        ..Default::default()
    })?;
    driver.adv_start(
        sys::BLE_OWN_ADDR_PUBLIC as u8,
        &BleAdvParams {
            conn_mode: sys::BLE_GAP_CONN_MODE_UND as u8,
            disc_mode: sys::BLE_GAP_DISC_MODE_GEN as u8,
            itvl_min: 0x30,
            itvl_max: 0x60,
            ..Default::default()
        },
    )
}
