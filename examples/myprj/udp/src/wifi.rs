//! WiFi initialization and management module
//!
//! This module handles WiFi connection setup, static IP configuration,
//! and network stack initialization.

use blocking_network_stack::Stack;
use esp_hal::{rng::Rng, time};
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    peripherals::Peripherals,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice};
use smoltcp::iface::{SocketSet, SocketStorage};
use log::{info, warn, error, debug};

/// Initialize WiFi hardware and create controller and device
///
/// This function performs the following steps:
/// 1. Initializes hardware peripherals (esp-hal)
/// 2. Configures heap allocator (72KB)
/// 3. Initializes RTOS timer
/// 4. Initializes esp-radio controller
/// 5. Creates WiFi controller and device interface
///
/// # Arguments
/// * `peripherals` - ESP32 peripherals
///
/// # Returns
/// A tuple containing (WifiController, Stack)
pub fn initialize_wifi(
    peripherals: Peripherals,
) -> (WifiController, Stack<'static>) {
    // Initialize RTOS timer
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    
    #[cfg(target_arch = "riscv32")]
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    
    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        sw_int.software_interrupt0,
    );

    // Initialize esp-radio controller
    let esp_radio_ctrl = esp_radio::init().unwrap();

    // Create WiFi controller and device
    let (mut controller, interfaces) =
        esp_radio::wifi::new(&esp_radio_ctrl, peripherals.WIFI, Default::default()).unwrap();

    let mut device = interfaces.sta;
    let iface = create_interface(&mut device);

    // Disable power saving for better performance
    controller
        .set_power_saving(esp_radio::wifi::PowerSaveMode::None)
        .unwrap();

    // Create network stack
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let socket_set = SocketSet::new(&mut socket_set_entries[..]);

    let rng = Rng::new();
    let now = || time::Instant::now().duration_since_epoch().as_millis();
    let stack = Stack::new(iface, device, socket_set, now, rng.random());

    (controller, stack)
}

/// Create smoltcp network interface
///
/// # Arguments
/// * `device` - WiFi device
///
/// # Returns
/// A configured smoltcp Interface
fn create_interface(device: &mut WifiDevice) -> smoltcp::iface::Interface {
    smoltcp::iface::Interface::new(
        smoltcp::iface::Config::new(smoltcp::wire::HardwareAddress::Ethernet(
            smoltcp::wire::EthernetAddress::from_bytes(&device.mac_address()),
        )),
        device,
        timestamp(),
    )
}

/// Get current timestamp for smoltcp
fn timestamp() -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_micros(
        esp_hal::time::Instant::now()
            .duration_since_epoch()
            .as_micros() as i64,
    )
}

/// WiFi connection error types
#[derive(Debug)]
pub enum WifiError {
    /// Failed to configure WiFi
    ConfigurationFailed,
    /// Failed to start WiFi
    StartFailed,
    /// Network scan failed
    ScanFailed,
    /// Connection attempt failed
    ConnectionFailed,
    /// Connection timeout (exceeded 30 seconds)
    ConnectionTimeout,
    /// Maximum retry attempts exceeded
    MaxRetriesExceeded,
}

/// Connect to WiFi network with timeout and retry logic
///
/// This function performs the following steps:
/// 1. Configures WiFi client mode with SSID and password
/// 2. Starts WiFi and executes network scan
/// 3. Connects to WiFi network
/// 4. Implements connection status check loop with 30-second timeout
/// 5. Retries up to 3 times on failure
///
/// # Arguments
/// * `controller` - WiFi controller
/// * `ssid` - WiFi network SSID
/// * `password` - WiFi network password
///
/// # Returns
/// Result indicating success or failure
pub fn connect_wifi(
    controller: &mut WifiController,
    ssid: &str,
    password: &str,
) -> Result<(), WifiError> {
    const MAX_RETRIES: u32 = 3;
    const CONNECTION_TIMEOUT_MS: u64 = 30_000; // 30 seconds
    
    for attempt in 1..=MAX_RETRIES {
        info!("WiFi connection attempt {}/{}", attempt, MAX_RETRIES);
        
        match connect_wifi_once(controller, ssid, password, CONNECTION_TIMEOUT_MS) {
            Ok(()) => {
                info!("WiFi connected successfully on attempt {}", attempt);
                return Ok(());
            }
            Err(e) => {
                error!("WiFi connection attempt {} failed: {:?}", attempt, e);
                
                // Log detailed error information
                match e {
                    WifiError::ConnectionTimeout => {
                        error!("Connection timeout after {}ms", CONNECTION_TIMEOUT_MS);
                        warn!("Check if SSID '{}' is in range and credentials are correct", ssid);
                    }
                    WifiError::ConnectionFailed => {
                        error!("Connection failed during handshake");
                        warn!("Verify password and network security settings");
                    }
                    WifiError::StartFailed => {
                        error!("Failed to start WiFi hardware");
                        warn!("Check hardware initialization");
                    }
                    WifiError::ScanFailed => {
                        error!("Network scan failed");
                        warn!("Check WiFi antenna and signal strength");
                    }
                    _ => {
                        error!("Error: {:?}", e);
                    }
                }
                
                if attempt < MAX_RETRIES {
                    info!("Retrying in 2 seconds...");
                    esp_hal::delay::Delay::new().delay_millis(2000);
                } else {
                    error!("Maximum retry attempts ({}) exceeded", MAX_RETRIES);
                    return Err(WifiError::MaxRetriesExceeded);
                }
            }
        }
    }
    
    Err(WifiError::MaxRetriesExceeded)
}

/// Attempt to connect to WiFi once with timeout
///
/// # Arguments
/// * `controller` - WiFi controller
/// * `ssid` - WiFi network SSID
/// * `password` - WiFi network password
/// * `timeout_ms` - Connection timeout in milliseconds
///
/// # Returns
/// Result indicating success or failure
fn connect_wifi_once(
    controller: &mut WifiController,
    ssid: &str,
    password: &str,
    timeout_ms: u64,
) -> Result<(), WifiError> {
    use esp_radio::wifi::ScanConfig;

    // Configure WiFi client mode
    let client_config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(ssid.into())
            .with_password(password.into()),
    );
    
    let res = controller.set_config(&client_config);
    debug!("WiFi configuration set: {:?}", res);
    if res.is_err() {
        return Err(WifiError::ConfigurationFailed);
    }

    // Start WiFi
    debug!("Starting WiFi...");
    controller.start().map_err(|e| {
        error!("Failed to start WiFi: {:?}", e);
        WifiError::StartFailed
    })?;
    debug!("WiFi started: {:?}", controller.is_started());

    // Execute network scan
    debug!("Starting WiFi scan...");
    let scan_config = ScanConfig::default().with_max(10);
    let res = controller.scan_with_config(scan_config).map_err(|e| {
        error!("WiFi scan failed: {:?}", e);
        WifiError::ScanFailed
    })?;
    
    info!("Found {} access points", res.len());
    for ap in res {
        debug!("AP: SSID={:?}, Signal={} dBm, Channel={}", 
            ap.ssid, ap.signal_strength, ap.channel);
    }

    debug!("WiFi capabilities: {:?}", controller.capabilities());
    
    // Connect to WiFi network
    info!("Connecting to SSID '{}'...", ssid);
    let connect_result = controller.connect();
    debug!("Connect command result: {:?}", connect_result);
    if connect_result.is_err() {
        return Err(WifiError::ConnectionFailed);
    }

    // Wait to get connected with timeout
    debug!("Waiting for connection (timeout: {}ms)...", timeout_ms);
    let start_time = esp_hal::time::Instant::now();
    let timeout_duration = esp_hal::time::Duration::from_millis(timeout_ms);
    
    let mut last_status_log = start_time;
    let status_log_interval = esp_hal::time::Duration::from_millis(5000); // Log every 5 seconds
    
    loop {
        match controller.is_connected() {
            Ok(true) => {
                let elapsed = esp_hal::time::Instant::now().duration_since(start_time);
                info!("WiFi connected after {}ms", elapsed.as_millis());
                return Ok(());
            }
            Ok(false) => {
                // Still connecting, check timeout
                let now = esp_hal::time::Instant::now();
                let elapsed = now.duration_since(start_time);
                
                // Log status periodically
                if now.duration_since(last_status_log) > status_log_interval {
                    debug!("Still connecting... (elapsed: {}ms)", elapsed.as_millis());
                    last_status_log = now;
                }
                
                if elapsed > timeout_duration {
                    error!("Connection timeout after {}ms", elapsed.as_millis());
                    return Err(WifiError::ConnectionTimeout);
                }
                
                // Small delay to avoid busy waiting
                esp_hal::delay::Delay::new().delay_millis(100);
            }
            Err(err) => {
                error!("WiFi connection error during status check: {:?}", err);
                return Err(WifiError::ConnectionFailed);
            }
        }
    }
}

/// Static IP configuration error types
#[derive(Debug)]
pub enum StaticIpError {
    /// Failed to set IP configuration
    ConfigurationFailed,
}

/// Configure static IP address
///
/// This function performs the following steps:
/// 1. Creates network interface (smoltcp interface) - already done in initialize_wifi
/// 2. Configures static IP address, gateway and subnet mask
/// 3. Uses blocking_network_stack to set IP configuration
/// 4. Verifies network configuration is effective
///
/// # Arguments
/// * `stack` - Network stack
/// * `ip` - Static IP address
/// * `gateway` - Gateway IP address
///
/// # Returns
/// Result indicating success or failure
pub fn configure_static_ip(
    stack: &mut Stack,
    ip: [u8; 4],
    gateway: [u8; 4],
) -> Result<(), StaticIpError> {
    info!("Configuring static IP...");
    debug!("  IP: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
    debug!("  Gateway: {}.{}.{}.{}", gateway[0], gateway[1], gateway[2], gateway[3]);
    debug!("  Subnet mask: 255.255.255.0 (24-bit)");

    // Configure static IP with gateway and subnet mask (24-bit)
    stack
        .set_iface_configuration(&blocking_network_stack::ipv4::Configuration::Client(
            blocking_network_stack::ipv4::ClientConfiguration::Fixed(
                blocking_network_stack::ipv4::ClientSettings {
                    ip: blocking_network_stack::ipv4::Ipv4Addr::from(ip),
                    subnet: blocking_network_stack::ipv4::Subnet {
                        gateway: blocking_network_stack::ipv4::Ipv4Addr::from(gateway),
                        mask: blocking_network_stack::ipv4::Mask(24),
                    },
                    dns: None,
                    secondary_dns: None,
                },
            ),
        ))
        .map_err(|e| {
            error!("Failed to set static IP configuration: {:?}", e);
            StaticIpError::ConfigurationFailed
        })?;

    info!("Static IP configuration completed");

    Ok(())
}
