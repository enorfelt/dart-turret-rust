use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{
    AccessPointConfiguration, AuthMethod, BlockingWifi, Configuration, EspWifi,
};

pub const SSID: &str = "DART Vision";
pub const CHANNEL: u8 = 1;
const MAX_CLIENTS: u16 = 4;

pub fn start_access_point(
    modem: Modem<'static>,
    password: &str,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), Some(nvs))?, sys_loop)?;
    wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: SSID.try_into().expect("SSID fits"),
        password: password.try_into().expect("password fits"),
        auth_method: AuthMethod::WPA2Personal,
        channel: CHANNEL,
        max_connections: MAX_CLIENTS,
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.wait_netif_up()?;
    let ip = wifi.wifi().ap_netif().get_ip_info()?.ip;
    log::info!("softAP {SSID} on channel {CHANNEL}, http://{ip}/");
    Ok(wifi)
}
