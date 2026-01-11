//! UDP Client module
//!
//! This module implements UDP client functionality including:
//! - Client configuration
//! - Data packet sending
//! - Response reception
//! - Client main loop with periodic sending

use blocking_network_stack::Stack;
use smoltcp::socket::{udp, UdpSocket};
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};
use log::{info, warn, error, debug};

/// Maximum UDP payload size (Ethernet MTU 1500 - IP header 20 - UDP header 8)
pub const MAX_UDP_PAYLOAD: usize = 1472;

/// UDP Client error types
#[derive(Debug)]
pub enum UdpClientError {
    /// Failed to create socket
    SocketCreationFailed,
    /// Failed to send data
    SendFailed,
    /// Failed to receive response
    ReceiveFailed,
    /// Response timeout
    Timeout,
    /// Invalid configuration
    InvalidConfig,
    /// Packet too large (exceeds MAX_UDP_PAYLOAD)
    PacketTooLarge(usize),
}

/// UDP Client configuration
#[derive(Debug, Clone, Copy)]
pub struct UdpClientConfig {
    /// Target server IP address
    pub target_ip: [u8; 4],
    /// Target server port
    pub target_port: u16,
}

impl UdpClientConfig {
    /// Create a new UDP client configuration
    ///
    /// # Arguments
    /// * `target_ip` - Target server IP address
    /// * `target_port` - Target server port
    ///
    /// # Returns
    /// A new UdpClientConfig instance
    pub fn new(target_ip: [u8; 4], target_port: u16) -> Self {
        Self {
            target_ip,
            target_port,
        }
    }

    /// Get the target endpoint
    ///
    /// # Returns
    /// IpEndpoint for the target server
    pub fn target_endpoint(&self) -> IpEndpoint {
        IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::from_bytes(&self.target_ip)),
            self.target_port,
        )
    }
}


/// Create a UDP socket for client use
///
/// This function creates a UDP socket with appropriate buffer sizes for client operations.
///
/// # Arguments
/// * `stack` - Network stack
///
/// # Returns
/// Result containing the socket handle or an error
pub fn create_udp_client_socket(
    stack: &mut Stack,
) -> Result<smoltcp::iface::SocketHandle, UdpClientError> {
    debug!("Creating UDP client socket");

    // Create UDP socket with appropriate buffer sizes
    let rx_buffer = udp::PacketBuffer::new(
        vec![udp::PacketMetadata::EMPTY; 4],
        vec![0u8; MAX_UDP_PAYLOAD * 4],
    );
    let tx_buffer = udp::PacketBuffer::new(
        vec![udp::PacketMetadata::EMPTY; 4],
        vec![0u8; MAX_UDP_PAYLOAD * 4],
    );

    let socket = UdpSocket::new(rx_buffer, tx_buffer);

    // Add socket to the stack
    let socket_handle = stack.sockets_mut().add(socket);

    debug!("UDP client socket created with handle: {:?}", socket_handle);
    Ok(socket_handle)
}

/// Construct a test data packet with timestamp and sequence number
///
/// # Arguments
/// * `sequence` - Packet sequence number
///
/// # Returns
/// A vector containing the packet data
pub fn construct_test_packet(sequence: u32) -> heapless::Vec<u8, 256> {
    use heapless::Vec;

    let mut packet = Vec::new();

    // Get current timestamp
    let timestamp = esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis();

    // Format: "UDP Test Packet #<seq> at <timestamp>ms"
    let msg = heapless::String::<256>::from("UDP Test Packet #");
    packet.extend_from_slice(msg.as_bytes()).ok();

    // Add sequence number
    let seq_str = heapless::String::<32>::from(sequence);
    packet.extend_from_slice(seq_str.as_bytes()).ok();

    packet.extend_from_slice(b" at ").ok();

    // Add timestamp
    let ts_str = heapless::String::<32>::from(timestamp);
    packet.extend_from_slice(ts_str.as_bytes()).ok();

    packet.extend_from_slice(b"ms").ok();

    packet
}

/// Send UDP data packet to target server
///
/// This function sends a data packet to the configured target server and logs
/// the sending information. It validates packet size before sending.
///
/// # Arguments
/// * `stack` - Network stack
/// * `socket_handle` - Handle to the UDP socket
/// * `data` - Data to send
/// * `config` - Client configuration with target endpoint
///
/// # Returns
/// Result containing bytes sent or an error
pub fn send_udp_packet(
    stack: &mut Stack,
    socket_handle: smoltcp::iface::SocketHandle,
    data: &[u8],
    config: &UdpClientConfig,
) -> Result<usize, UdpClientError> {
    // Validate packet size
    if data.len() > MAX_UDP_PAYLOAD {
        error!("Packet too large: {} > {} bytes", data.len(), MAX_UDP_PAYLOAD);
        error!("  Action: Packet rejected");
        return Err(UdpClientError::PacketTooLarge(data.len()));
    }

    // Process network events first
    stack.work();

    // Get the socket
    let socket = stack
        .sockets_mut()
        .get_mut::<UdpSocket>(socket_handle);

    // Get target endpoint
    let target = config.target_endpoint();

    // Send data
    match socket.send_slice(data, target) {
        Ok(_) => {
            let timestamp = esp_hal::time::Instant::now()
                .duration_since_epoch()
                .as_millis();

            info!("[{}ms] Sent {} bytes to {}:{}", 
                timestamp, data.len(), target.addr, target.port);

            // Log packet content if it's text
            if let Ok(text) = core::str::from_utf8(data) {
                debug!("  Content: {}", text);
            } else {
                debug!("  Binary data (first 16 bytes): {:02x?}", 
                    &data[..data.len().min(16)]);
            }

            Ok(data.len())
        }
        Err(e) => {
            error!("Failed to send UDP packet: {:?}", e);
            error!("  Destination: {}:{}", target.addr, target.port);
            error!("  Packet size: {} bytes", data.len());
            warn!("Check network connectivity and socket state");
            Err(UdpClientError::SendFailed)
        }
    }
}


/// Receive UDP response from server with timeout
///
/// This function waits for a response from the server with a specified timeout.
/// It processes network events and checks for incoming data.
///
/// # Arguments
/// * `stack` - Network stack
/// * `socket_handle` - Handle to the UDP socket
/// * `buffer` - Buffer to store received data
/// * `timeout_ms` - Timeout in milliseconds
///
/// # Returns
/// Result containing (bytes_received, source_endpoint) or an error
pub fn receive_udp_response(
    stack: &mut Stack,
    socket_handle: smoltcp::iface::SocketHandle,
    buffer: &mut [u8],
    timeout_ms: u64,
) -> Result<(usize, IpEndpoint), UdpClientError> {
    let start_time = esp_hal::time::Instant::now();
    let timeout_duration = esp_hal::time::Duration::from_millis(timeout_ms);

    loop {
        // Process network events
        stack.work();

        // Get the socket
        let socket = stack
            .sockets_mut()
            .get_mut::<UdpSocket>(socket_handle);

        // Check if data is available
        if socket.can_recv() {
            // Receive data
            match socket.recv_slice(buffer) {
                Ok((size, endpoint)) => {
                    let timestamp = esp_hal::time::Instant::now()
                        .duration_since_epoch()
                        .as_millis();

                    info!("[{}ms] Received {} bytes from {}:{}", 
                        timestamp, size, endpoint.addr, endpoint.port);

                    // Try to parse as UTF-8 text
                    if let Ok(text) = core::str::from_utf8(&buffer[..size]) {
                        debug!("  Response: {}", text);
                    } else {
                        debug!("  Binary response (first 16 bytes): {:02x?}", 
                            &buffer[..size.min(16)]);
                    }

                    return Ok((size, endpoint));
                }
                Err(e) => {
                    error!("Failed to receive response: {:?}", e);
                    return Err(UdpClientError::ReceiveFailed);
                }
            }
        }

        // Check timeout
        let elapsed = esp_hal::time::Instant::now().duration_since(start_time);
        if elapsed > timeout_duration {
            warn!("Response timeout after {}ms", timeout_ms);
            warn!("Check if server is running and reachable");
            return Err(UdpClientError::Timeout);
        }

        // Small delay to avoid busy waiting
        esp_hal::delay::Delay::new().delay_millis(10);
    }
}

/// Verify response content
///
/// This function performs basic validation on the received response.
/// For this implementation, we check that the response is not empty
/// and optionally that it contains expected acknowledgment patterns.
///
/// # Arguments
/// * `response_data` - Received response data
///
/// # Returns
/// true if response is valid, false otherwise
pub fn verify_response(response_data: &[u8]) -> bool {
    // Basic validation: response should not be empty
    if response_data.is_empty() {
        return false;
    }

    // Check if response contains "ACK" pattern (common acknowledgment)
    if let Ok(text) = core::str::from_utf8(response_data) {
        if text.contains("ACK") {
            return true;
        }
    }

    // If not empty and we got here, consider it valid
    // (server might send different response format)
    true
}


/// Client statistics
#[derive(Debug, Default)]
pub struct ClientStats {
    /// Total packets sent
    pub packets_sent: u32,
    /// Successful sends
    pub send_success: u32,
    /// Failed sends
    pub send_failed: u32,
    /// Responses received
    pub responses_received: u32,
    /// Response timeouts
    pub response_timeouts: u32,
}

impl ClientStats {
    /// Print statistics summary
    pub fn print_summary(&self) {
        info!("=== Client Statistics ===");
        info!("  Packets sent: {}", self.packets_sent);
        info!("  Send success: {}", self.send_success);
        info!("  Send failed: {}", self.send_failed);
        info!("  Responses received: {}", self.responses_received);
        info!("  Response timeouts: {}", self.response_timeouts);
        
        if self.packets_sent > 0 {
            let success_rate = (self.send_success * 100) / self.packets_sent;
            info!("  Success rate: {}%", success_rate);
        }
    }
}

/// Run UDP client main loop
///
/// This function implements the main client loop that:
/// 1. Periodically sends data packets (every 5 seconds)
/// 2. Waits for and receives server responses
/// 3. Handles send failures and timeouts
/// 4. Records statistics (send success/failure counts)
/// 5. Prints periodic statistics summaries
///
/// # Arguments
/// * `stack` - Network stack
/// * `config` - Client configuration
pub fn run_client_loop(
    stack: &mut Stack,
    config: UdpClientConfig,
) -> ! {
    info!("UDP Client started");
    info!("Target: {}.{}.{}.{}:{}", 
        config.target_ip[0], config.target_ip[1], 
        config.target_ip[2], config.target_ip[3], 
        config.target_port);

    // Create UDP socket
    let socket_handle = match create_udp_client_socket(stack) {
        Ok(handle) => handle,
        Err(e) => {
            error!("FATAL: Failed to create UDP socket: {:?}", e);
            error!("System halted.");
            loop {}
        }
    };

    let mut stats = ClientStats::default();
    let mut sequence = 0u32;
    let mut buffer = [0u8; MAX_UDP_PAYLOAD];

    // Send interval: 5 seconds
    let send_interval_ms = 5000u64;
    // Response timeout: 2 seconds
    let response_timeout_ms = 2000u64;
    // Statistics print interval: 10 packets
    let stats_interval = 10u32;

    info!("Starting client loop (sending every {}ms)...", send_interval_ms);

    loop {
        sequence += 1;
        stats.packets_sent += 1;

        info!("\n--- Packet #{} ---", sequence);

        // Construct test packet
        let packet = construct_test_packet(sequence);

        // Send packet
        match send_udp_packet(stack, socket_handle, &packet, &config) {
            Ok(sent) => {
                stats.send_success += 1;
                info!("Send successful ({} bytes)", sent);

                // Wait for response
                match receive_udp_response(stack, socket_handle, &mut buffer, response_timeout_ms) {
                    Ok((size, source)) => {
                        stats.responses_received += 1;
                        
                        // Verify response
                        if verify_response(&buffer[..size]) {
                            info!("Response verified successfully from {}:{}", 
                                source.addr, source.port);
                        } else {
                            warn!("Response verification failed");
                        }
                    }
                    Err(UdpClientError::Timeout) => {
                        stats.response_timeouts += 1;
                        warn!("No response received (timeout)");
                    }
                    Err(e) => {
                        error!("Error receiving response: {:?}", e);
                    }
                }
            }
            Err(UdpClientError::PacketTooLarge(size)) => {
                stats.send_failed += 1;
                error!("Packet too large ({} bytes, max: {})", size, MAX_UDP_PAYLOAD);
            }
            Err(e) => {
                stats.send_failed += 1;
                error!("Send failed: {:?}", e);
            }
        }

        // Print statistics periodically
        if sequence % stats_interval == 0 {
            info!("");
            stats.print_summary();
            info!("");
        }

        // Wait before sending next packet
        debug!("Waiting {}ms before next packet...", send_interval_ms);
        esp_hal::delay::Delay::new().delay_millis(send_interval_ms as u32);
    }
}
