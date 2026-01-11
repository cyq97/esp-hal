//! UDP Server module
//!
//! This module implements UDP server functionality including:
//! - Socket creation and port binding
//! - Packet reception and parsing
//! - Response sending
//! - Continuous packet processing

use blocking_network_stack::Stack;
use smoltcp::socket::{udp, UdpSocket};
use smoltcp::wire::IpEndpoint;
use log::{info, warn, error, debug};

/// Maximum UDP payload size (Ethernet MTU 1500 - IP header 20 - UDP header 8)
pub const MAX_UDP_PAYLOAD: usize = 1472;

/// UDP Server error types
#[derive(Debug)]
pub enum UdpServerError {
    /// Port is already in use
    PortInUse,
    /// Failed to bind to port
    BindFailed,
    /// Failed to receive data
    ReceiveFailed,
    /// Failed to send data
    SendFailed,
    /// Buffer too small
    BufferTooSmall,
    /// Packet too large (exceeds MAX_UDP_PAYLOAD)
    PacketTooLarge(usize),
    /// Receive timeout
    ReceiveTimeout,
}

/// Create and bind a UDP socket to the specified port
///
/// This function creates a UDP socket and attempts to bind it to the specified port.
/// If the port is already in use, it will try the next port (port + 1).
///
/// # Arguments
/// * `stack` - Network stack
/// * `port` - Desired port number
///
/// # Returns
/// Result containing the bound port number or an error
pub fn bind_udp_server(
    stack: &mut Stack,
    port: u16,
) -> Result<u16, UdpServerError> {
    info!("Binding UDP server...");

    // Try to bind to the requested port
    let mut current_port = port;
    let max_attempts = 2; // Try original port and one backup

    for attempt in 0..max_attempts {
        debug!("Attempt {}/{}: Binding to port {}", attempt + 1, max_attempts, current_port);

        // Create UDP socket with appropriate buffer sizes
        let rx_buffer = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY; 4],
            vec![0u8; MAX_UDP_PAYLOAD * 4],
        );
        let tx_buffer = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY; 4],
            vec![0u8; MAX_UDP_PAYLOAD * 4],
        );

        let mut socket = UdpSocket::new(rx_buffer, tx_buffer);

        // Try to bind to the current port
        match socket.bind(current_port) {
            Ok(_) => {
                info!("Successfully bound UDP socket to port {}", current_port);
                
                // Add socket to the stack
                let socket_handle = stack.sockets_mut().add(socket);
                
                debug!("UDP socket handle: {:?}", socket_handle);
                return Ok(current_port);
            }
            Err(e) => {
                warn!("Failed to bind to port {}: {:?}", current_port, e);
                
                if attempt < max_attempts - 1 {
                    // Try next port
                    current_port += 1;
                    info!("Trying backup port {}", current_port);
                } else {
                    error!("All port binding attempts failed");
                    warn!("Check if another process is using ports {}-{}", port, current_port);
                    return Err(UdpServerError::PortInUse);
                }
            }
        }
    }

    Err(UdpServerError::BindFailed)
}

/// Receive UDP packet from the socket
///
/// This function receives a UDP packet from the socket and returns the data
/// along with the source address. It validates packet size and rejects packets
/// that exceed MAX_UDP_PAYLOAD.
///
/// # Arguments
/// * `stack` - Network stack
/// * `socket_handle` - Handle to the UDP socket
/// * `buffer` - Buffer to store received data
///
/// # Returns
/// Result containing (bytes_received, source_endpoint) or an error
pub fn receive_udp_packet(
    stack: &mut Stack,
    socket_handle: smoltcp::iface::SocketHandle,
    buffer: &mut [u8],
) -> Result<(usize, IpEndpoint), UdpServerError> {
    // Process network events
    stack.work();

    // Get the socket
    let socket = stack
        .sockets_mut()
        .get_mut::<UdpSocket>(socket_handle);

    // Check if data is available
    if !socket.can_recv() {
        return Err(UdpServerError::ReceiveTimeout);
    }

    // Receive data
    match socket.recv_slice(buffer) {
        Ok((size, endpoint)) => {
            // Validate packet size
            if size > MAX_UDP_PAYLOAD {
                warn!("Received packet exceeds maximum size: {} > {} bytes", 
                    size, MAX_UDP_PAYLOAD);
                warn!("  Source: {}:{}", endpoint.addr, endpoint.port);
                warn!("  Action: Packet rejected");
                return Err(UdpServerError::PacketTooLarge(size));
            }

            // Get current timestamp
            let timestamp = esp_hal::time::Instant::now()
                .duration_since_epoch()
                .as_millis();

            // Log received packet information
            info!("[{}ms] Received {} bytes from {}:{}", 
                timestamp, size, endpoint.addr, endpoint.port);

            // Try to parse as UTF-8 text
            if let Ok(text) = core::str::from_utf8(&buffer[..size]) {
                debug!("  Content type: UTF-8 text");
                debug!("  Text: {}", text);
            } else {
                debug!("  Content type: Binary data");
                debug!("  First 16 bytes: {:02x?}", &buffer[..size.min(16)]);
            }

            Ok((size, endpoint))
        }
        Err(e) => {
            error!("Failed to receive UDP packet: {:?}", e);
            Err(UdpServerError::ReceiveFailed)
        }
    }
}

/// Parse received UDP packet content
///
/// This function attempts to parse the packet as UTF-8 text or treats it as binary data.
///
/// # Arguments
/// * `data` - Received packet data
///
/// # Returns
/// A string describing the packet content
pub fn parse_packet_content(data: &[u8]) -> heapless::String<256> {
    use heapless::String;
    
    let mut result = String::new();
    
    if let Ok(text) = core::str::from_utf8(data) {
        // UTF-8 text
        let _ = core::fmt::write(&mut result, format_args!("Text: {}", text));
    } else {
        // Binary data - show first few bytes
        let _ = core::fmt::write(&mut result, format_args!("Binary ({} bytes)", data.len()));
    }
    
    result
}

/// Send UDP response to the source address
///
/// This function constructs an acknowledgment response message and sends it
/// back to the source address. It handles send failures with detailed error logging.
///
/// # Arguments
/// * `stack` - Network stack
/// * `socket_handle` - Handle to the UDP socket
/// * `original_data` - Original received data
/// * `source` - Source endpoint to send response to
///
/// # Returns
/// Result containing bytes sent or an error
pub fn send_udp_response(
    stack: &mut Stack,
    socket_handle: smoltcp::iface::SocketHandle,
    original_data: &[u8],
    source: IpEndpoint,
) -> Result<usize, UdpServerError> {
    // Construct acknowledgment response
    let mut response = heapless::Vec::<u8, 512>::new();
    
    // Add acknowledgment header
    let ack_msg = b"ACK: Received ";
    response.extend_from_slice(ack_msg).ok();
    
    // Add data size
    let size_str = heapless::String::<32>::from(original_data.len());
    response.extend_from_slice(size_str.as_bytes()).ok();
    response.extend_from_slice(b" bytes").ok();
    
    // Add data digest (first 32 bytes or less)
    if original_data.len() > 0 {
        response.extend_from_slice(b" [").ok();
        let digest_len = original_data.len().min(32);
        
        // Try to show as text if valid UTF-8
        if let Ok(text) = core::str::from_utf8(&original_data[..digest_len]) {
            response.extend_from_slice(text.as_bytes()).ok();
        } else {
            // Show as hex for binary data
            for (i, byte) in original_data[..digest_len].iter().enumerate() {
                if i > 0 && i % 8 == 0 {
                    break; // Limit to 8 bytes for hex display
                }
                let hex = heapless::String::<4>::from(*byte);
                response.extend_from_slice(hex.as_bytes()).ok();
                if i < digest_len - 1 {
                    response.extend_from_slice(b" ").ok();
                }
            }
        }
        
        if original_data.len() > digest_len {
            response.extend_from_slice(b"...").ok();
        }
        response.extend_from_slice(b"]").ok();
    }

    // Get the socket
    let socket = stack
        .sockets_mut()
        .get_mut::<UdpSocket>(socket_handle);

    // Send response
    match socket.send_slice(&response, source) {
        Ok(_) => {
            let timestamp = esp_hal::time::Instant::now()
                .duration_since_epoch()
                .as_millis();
            
            info!("[{}ms] Sent {} bytes response to {}:{}", 
                timestamp, response.len(), source.addr, source.port);
            
            Ok(response.len())
        }
        Err(e) => {
            error!("Failed to send UDP response: {:?}", e);
            error!("  Destination: {}:{}", source.addr, source.port);
            error!("  Response size: {} bytes", response.len());
            warn!("Check network connectivity and socket state");
            Err(UdpServerError::SendFailed)
        }
    }
}


/// Run UDP server main loop
///
/// This function implements the main server loop that continuously:
/// 1. Processes network events via stack.work()
/// 2. Checks for incoming UDP packets
/// 3. Receives and parses packets
/// 4. Sends acknowledgment responses
/// 5. Implements appropriate delays to avoid high CPU usage
///
/// # Arguments
/// * `stack` - Network stack
/// * `socket_handle` - Handle to the UDP socket
pub fn run_server_loop(
    stack: &mut Stack,
    socket_handle: smoltcp::iface::SocketHandle,
) -> ! {
    info!("UDP Server started - listening for packets...");
    
    let mut buffer = [0u8; MAX_UDP_PAYLOAD];
    let mut packet_count = 0u32;
    let mut error_count = 0u32;

    loop {
        // Process network events
        stack.work();

        // Try to receive a packet
        match receive_udp_packet(stack, socket_handle, &mut buffer) {
            Ok((size, source)) => {
                packet_count += 1;
                info!("Packet #{}: {} bytes from {}:{}", 
                    packet_count, size, source.addr, source.port);

                // Send acknowledgment response
                match send_udp_response(stack, socket_handle, &buffer[..size], source) {
                    Ok(sent) => {
                        debug!("Response sent successfully ({} bytes)", sent);
                    }
                    Err(e) => {
                        error_count += 1;
                        error!("Failed to send response: {:?}", e);
                        warn!("Total errors: {}", error_count);
                    }
                }
            }
            Err(UdpServerError::ReceiveTimeout) => {
                // No data available, continue loop (this is normal)
            }
            Err(UdpServerError::PacketTooLarge(size)) => {
                error_count += 1;
                warn!("Rejected oversized packet ({} bytes, max: {})", 
                    size, MAX_UDP_PAYLOAD);
                warn!("Total errors: {}", error_count);
            }
            Err(e) => {
                error_count += 1;
                error!("Error receiving packet: {:?}", e);
                warn!("Total errors: {}", error_count);
            }
        }

        // Small delay to avoid excessive CPU usage
        // This allows other tasks to run and reduces power consumption
        esp_hal::delay::Delay::new().delay_millis(10);
    }
}

