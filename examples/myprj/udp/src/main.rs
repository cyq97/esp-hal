#![no_std]
#![no_main]

//! UDP Communication Example
//!
//! This example demonstrates UDP client and server functionality
//! using static IP configuration on ESP32 series chips.
//!
//! # Architecture
//!
//! The system is organized into the following modules:
//! - `config`: Configuration management from environment variables
//! - `wifi`: WiFi initialization and static IP configuration
//! - `udp_server`: UDP server implementation (receive and respond)
//! - `udp_client`: UDP client implementation (send and receive)
//!
//! # Operation Modes
//!
//! The system supports two operation modes:
//!
//! ## Server Mode (Default)
//! - Binds to a UDP port (default: 8080)
//! - Listens for incoming UDP packets
//! - Sends acknowledgment responses
//! - Activated when TARGET_IP and TARGET_PORT are NOT set
//!
//! ## Client Mode
//! - Sends periodic UDP packets to a target server
//! - Waits for and validates responses
//! - Tracks statistics (success/failure counts)
//! - Activated when TARGET_IP and TARGET_PORT are set
//!
//! # Required Environment Variables
//!
//! - `SSID`: WiFi network name
//! - `PASSWORD`: WiFi password
//! - `STATIC_IP`: Device static IP (e.g., "192.168.1.100")
//! - `GATEWAY_IP`: Gateway IP (e.g., "192.168.1.1")
//!
//! # Optional Environment Variables
//!
//! - `UDP_PORT`: UDP listening port (default: 8080)
//! - `TARGET_IP`: Target server IP for client mode
//! - `TARGET_PORT`: Target server port for client mode
//! - `ESP_LOG_LEVEL`: Log level (ERROR, WARN, INFO, DEBUG)
//!
//! # Example Usage
//!
//! ## Server Mode
//! ```bash
//! export SSID="MyWiFi"
//! export PASSWORD="mypassword"
//! export STATIC_IP="192.168.1.100"
//! export GATEWAY_IP="192.168.1.1"
//! export UDP_PORT="8080"
//! cargo run --release
//! ```
//!
//! ## Client Mode
//! ```bash
//! export SSID="MyWiFi"
//! export PASSWORD="mypassword"
//! export STATIC_IP="192.168.1.101"
//! export GATEWAY_IP="192.168.1.1"
//! export TARGET_IP="192.168.1.100"
//! export TARGET_PORT="8080"
//! cargo run --release
//! ```
//!
//! # Requirements Mapping
//!
//! This implementation satisfies the following requirements:
//! - Requirements 1.1-1.4: WiFi connection and static IP configuration
//! - Requirements 2.1-2.4: UDP server functionality
//! - Requirements 3.1-3.4: UDP client functionality
//! - Requirements 4.1-4.4: Data format and protocol
//! - Requirements 5.1-5.4: Error handling and logging
//! - Requirements 6.1-6.4: Configuration management

mod config;
mod wifi;
mod udp_server;
mod udp_client;

use config::UdpConfig;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, main, peripherals::Peripherals};
use esp_radio::wifi::WifiController;
use blocking_network_stack::Stack;
use log::{info, error};

/// Main entry point for UDP communication system
///
/// This function performs the following initialization steps:
/// 1. Initializes logging system
/// 2. Loads configuration from environment variables
/// 3. Initializes WiFi and network stack
/// 4. Selects and runs appropriate mode (client or server)
///
/// The function never returns - it runs the selected mode loop indefinitely.
///
/// # Requirements
/// - Requirements 1.1: WiFi connection and initialization
/// - Requirements 6.1, 6.2: Configuration management
#[main]
fn main() -> ! {
    // Step 1: Initialize logging system
    // Set ESP_LOG_LEVEL environment variable to control log level:
    // - ERROR: Only errors
    // - WARN: Warnings and errors
    // - INFO: Info, warnings, and errors (default)
    // - DEBUG: All messages including debug
    initialize_logging();
    
    info!("=== UDP Communication System Starting ===");
    
    // Step 2: Initialize hardware and allocate heap
    let peripherals = initialize_hardware();
    
    // Step 3: Load configuration from environment variables
    let config = load_configuration();
    
    // Step 4: Initialize WiFi and network stack
    let (mut controller, mut stack) = initialize_network(peripherals);
    
    // Step 5: Connect to WiFi and configure static IP
    connect_and_configure_network(&mut controller, &mut stack, &config);
    
    info!("=== System Initialization Complete ===");
    
    // Step 6: Select and run appropriate mode based on configuration
    run_selected_mode(&mut stack, &config);
}

/// Initialize logging system
///
/// Initializes the logger from environment variable (default: INFO level).
fn initialize_logging() {
    esp_println::logger::init_logger_from_env();
}

/// Initialize hardware peripherals and heap allocator
///
/// # Returns
/// Initialized ESP32 peripherals
fn initialize_hardware() -> Peripherals {
    // Initialize hardware with max CPU clock
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    info!("Hardware initialized with CPU clock: {:?}", CpuClock::max());

    // Configure heap allocator (72KB)
    esp_alloc::heap_allocator!(size: 72 * 1024);
    info!("Heap allocator configured: 72KB");
    
    peripherals
}

/// Load configuration from environment variables
///
/// # Returns
/// Loaded UDP configuration
///
/// # Requirements
/// - Requirements 6.1: Read SSID, PASSWORD, STATIC_IP, GATEWAY_IP
/// - Requirements 6.2: Read UDP_PORT, TARGET_IP, TARGET_PORT
fn load_configuration() -> UdpConfig {
    let config = UdpConfig::load();
    
    info!("Configuration loaded:");
    info!("  SSID: {}", config.ssid);
    info!("  Static IP: {}.{}.{}.{}", 
        config.static_ip[0], config.static_ip[1], 
        config.static_ip[2], config.static_ip[3]);
    info!("  Gateway: {}.{}.{}.{}", 
        config.gateway_ip[0], config.gateway_ip[1], 
        config.gateway_ip[2], config.gateway_ip[3]);
    info!("  UDP Port: {}", config.udp_port);
    
    if config.is_client_mode() {
        let target_ip = config.target_ip.unwrap();
        let target_port = config.target_port.unwrap();
        info!("  Mode: Client");
        info!("  Target: {}.{}.{}.{}:{}", 
            target_ip[0], target_ip[1], target_ip[2], target_ip[3], target_port);
    } else {
        info!("  Mode: Server");
    }
    
    config
}

/// Initialize WiFi hardware and create network stack
///
/// # Arguments
/// * `peripherals` - ESP32 peripherals
///
/// # Returns
/// Tuple of (WifiController, Stack)
///
/// # Requirements
/// - Requirements 1.1: WiFi initialization
fn initialize_network(peripherals: Peripherals) -> (WifiController, Stack<'static>) {
    info!("Initializing WiFi and network stack...");
    wifi::initialize_wifi(peripherals)
}

/// Connect to WiFi network and configure static IP
///
/// # Arguments
/// * `controller` - WiFi controller
/// * `stack` - Network stack
/// * `config` - UDP configuration
///
/// # Requirements
/// - Requirements 1.1: WiFi connection
/// - Requirements 1.2, 1.3: Static IP configuration
/// - Requirements 1.4: Error handling and retry
fn connect_and_configure_network(
    controller: &mut WifiController,
    stack: &mut Stack,
    config: &UdpConfig,
) {
    // Connect to WiFi network
    info!("Connecting to WiFi network '{}'...", config.ssid);
    match wifi::connect_wifi(controller, config.ssid, config.password) {
        Ok(()) => {
            info!("WiFi connection established successfully");
        }
        Err(e) => {
            error!("FATAL: Failed to connect to WiFi: {:?}", e);
            error!("System halted. Please check configuration and restart.");
            loop {}
        }
    }
    
    // Configure static IP
    info!("Configuring static IP...");
    match wifi::configure_static_ip(stack, config.static_ip, config.gateway_ip) {
        Ok(()) => {
            info!("Network configuration complete");
        }
        Err(e) => {
            error!("FATAL: Failed to configure static IP: {:?}", e);
            error!("System halted. Please check configuration and restart.");
            loop {}
        }
    }
}

/// Select and run appropriate mode based on configuration
///
/// This function checks the configuration and runs either:
/// - Client mode: if TARGET_IP and TARGET_PORT are set
/// - Server mode: otherwise
///
/// # Arguments
/// * `stack` - Network stack
/// * `config` - UDP configuration
///
/// # Requirements
/// - Requirements 2.1: UDP server mode
/// - Requirements 3.1: UDP client mode
fn run_selected_mode(stack: &mut Stack, config: &UdpConfig) -> ! {
    if config.is_client_mode() {
        run_client_mode(stack, config)
    } else {
        run_server_mode(stack, config)
    }
}

/// Run UDP client mode
///
/// # Arguments
/// * `stack` - Network stack
/// * `config` - UDP configuration
///
/// # Requirements
/// - Requirements 3.1: UDP client configuration
fn run_client_mode(stack: &mut Stack, config: &UdpConfig) -> ! {
    info!("=== Starting UDP Client Mode ===");
    
    // Extract target IP and port (we know they exist because is_client_mode() returned true)
    let target_ip = config.target_ip.unwrap();
    let target_port = config.target_port.unwrap();
    
    info!("Target server: {}.{}.{}.{}:{}", 
        target_ip[0], target_ip[1], target_ip[2], target_ip[3], target_port);
    
    // Create client configuration
    let client_config = udp_client::UdpClientConfig::new(target_ip, target_port);
    
    // Run client loop (never returns)
    udp_client::run_client_loop(stack, client_config);
}

/// Run UDP server mode
///
/// # Arguments
/// * `stack` - Network stack
/// * `config` - UDP configuration
///
/// # Requirements
/// - Requirements 2.1: UDP server port binding
fn run_server_mode(stack: &mut Stack, config: &UdpConfig) -> ! {
    info!("=== Starting UDP Server Mode ===");
    info!("Listening on port {}", config.udp_port);
    
    match udp_server::bind_udp_server(stack, config.udp_port) {
        Ok(bound_port) => {
            info!("UDP server bound to port {}", bound_port);
            
            // Get the socket handle (it's the last one added)
            let socket_handle = stack.sockets().iter().last().unwrap().0;
            
            // Run server loop (never returns)
            udp_server::run_server_loop(stack, socket_handle);
        }
        Err(e) => {
            error!("FATAL: Failed to bind UDP server: {:?}", e);
            error!("System halted.");
            loop {}
        }
    }
}

// ============================================================================
// Module Integration Summary
// ============================================================================
//
// This main.rs file integrates all system modules:
//
// 1. Configuration Module (config.rs)
//    - Parses environment variables
//    - Validates IP addresses and ports
//    - Determines operation mode
//
// 2. WiFi Module (wifi.rs)
//    - Initializes WiFi hardware
//    - Connects to WiFi network with retry logic
//    - Configures static IP address
//    - Creates network stack
//
// 3. UDP Server Module (udp_server.rs)
//    - Binds UDP socket to port
//    - Receives UDP packets
//    - Parses packet content (text/binary)
//    - Sends acknowledgment responses
//    - Runs continuous server loop
//
// 4. UDP Client Module (udp_client.rs)
//    - Creates UDP client socket
//    - Constructs test packets with timestamps
//    - Sends packets to target server
//    - Receives and validates responses
//    - Tracks statistics
//    - Runs periodic client loop
//
// Error Handling:
// - WiFi connection errors: Retry up to 3 times with 30s timeout
// - Static IP configuration errors: Fatal, system halts
// - UDP server bind errors: Try backup port
// - UDP send/receive errors: Log and continue
// - Packet size validation: Reject packets > 1472 bytes
//
// All modules work together to provide a complete UDP communication system
// that can operate in either server or client mode based on configuration.
