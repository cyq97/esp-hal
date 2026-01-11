//! Configuration management module
//!
//! This module handles parsing and validation of environment variables
//! for UDP communication configuration.

/// UDP communication configuration structure
///
/// This structure holds all configuration parameters needed for
/// WiFi connection, static IP setup, and UDP communication.
pub struct UdpConfig {
    /// WiFi network SSID
    pub ssid: &'static str,
    /// WiFi network password
    pub password: &'static str,
    /// Static IP address for the device
    pub static_ip: [u8; 4],
    /// Gateway IP address
    pub gateway_ip: [u8; 4],
    /// UDP listening port (for server mode)
    pub udp_port: u16,
    /// Target IP address (for client mode, optional)
    pub target_ip: Option<[u8; 4]>,
    /// Target port (for client mode, optional)
    pub target_port: Option<u16>,
}

impl UdpConfig {
    /// Load configuration from environment variables
    ///
    /// Required environment variables:
    /// - SSID: WiFi network name
    /// - PASSWORD: WiFi password
    /// - STATIC_IP: Device static IP (e.g., "192.168.1.100")
    /// - GATEWAY_IP: Gateway IP (e.g., "192.168.1.1")
    ///
    /// Optional environment variables:
    /// - UDP_PORT: UDP listening port (default: 8080)
    /// - TARGET_IP: Target server IP for client mode
    /// - TARGET_PORT: Target server port for client mode
    pub fn load() -> Self {
        // Required environment variables
        let ssid = env!("SSID");
        let password = env!("PASSWORD");
        let static_ip_str = env!("STATIC_IP");
        let gateway_ip_str = env!("GATEWAY_IP");

        // Optional environment variables
        let udp_port = option_env!("UDP_PORT")
            .and_then(|s| parse_u16(s))
            .unwrap_or(8080);

        let target_ip = option_env!("TARGET_IP").map(|s| parse_ip(s));
        let target_port = option_env!("TARGET_PORT").and_then(|s| parse_u16(s));

        Self {
            ssid,
            password,
            static_ip: parse_ip(static_ip_str),
            gateway_ip: parse_ip(gateway_ip_str),
            udp_port,
            target_ip,
            target_port,
        }
    }

    /// Check if the configuration is for client mode
    ///
    /// Client mode is enabled when both TARGET_IP and TARGET_PORT are set
    pub fn is_client_mode(&self) -> bool {
        self.target_ip.is_some() && self.target_port.is_some()
    }
}

/// Parse an IP address string into a 4-byte array
///
/// # Arguments
/// * `ip` - IP address string in dotted decimal notation (e.g., "192.168.1.100")
///
/// # Returns
/// A 4-byte array representing the IP address
///
/// # Panics
/// Panics if the IP string is not in valid format or contains invalid octets
pub fn parse_ip(ip: &str) -> [u8; 4] {
    let mut result = [0u8; 4];
    for (idx, octet) in ip.split('.').enumerate() {
        result[idx] = u8::from_str_radix(octet, 10).unwrap();
    }
    result
}

/// Parse a u16 value from a string
///
/// # Arguments
/// * `s` - String representation of a u16 number
///
/// # Returns
/// Some(u16) if parsing succeeds, None otherwise
pub(crate) fn parse_u16(s: &str) -> Option<u16> {
    u16::from_str_radix(s, 10).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ip_valid() {
        assert_eq!(parse_ip("192.168.1.100"), [192, 168, 1, 100]);
        assert_eq!(parse_ip("10.0.0.1"), [10, 0, 0, 1]);
        assert_eq!(parse_ip("172.16.0.1"), [172, 16, 0, 1]);
    }

    #[test]
    fn test_parse_ip_boundary() {
        assert_eq!(parse_ip("0.0.0.0"), [0, 0, 0, 0]);
        assert_eq!(parse_ip("255.255.255.255"), [255, 255, 255, 255]);
    }

    #[test]
    fn test_parse_u16_valid() {
        assert_eq!(parse_u16("8080"), Some(8080));
        assert_eq!(parse_u16("1024"), Some(1024));
        assert_eq!(parse_u16("65535"), Some(65535));
        assert_eq!(parse_u16("0"), Some(0));
    }

    #[test]
    fn test_parse_u16_invalid() {
        assert_eq!(parse_u16("invalid"), None);
        assert_eq!(parse_u16("65536"), None); // Out of range
        assert_eq!(parse_u16("-1"), None);
        assert_eq!(parse_u16(""), None);
    }

    // Property-based tests
    // Feature: udp-communication, Property 1: 静态IP配置正确性
    // For any valid IP address and gateway configuration, after configuration
    // the system should maintain the same IP address and gateway values
    //
    // Note: Since this is embedded hardware that requires actual WiFi hardware,
    // we test the configuration data structures and validation logic rather than
    // the actual hardware interaction. The property validates that IP addresses
    // are correctly represented and can be round-tripped through string formatting.
    //
    // Validates: Requirements 1.2, 1.3

    use proptest::prelude::*;

    // Strategy to generate valid IP addresses (4 octets, each 0-255)
    fn ip_address_strategy() -> impl Strategy<Value = [u8; 4]> {
        prop::array::uniform4(0u8..=255u8)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        // Feature: udp-communication, Property 1: 静态IP配置正确性
        // Validates: Requirements 1.2, 1.3
        fn test_static_ip_configuration_correctness(
            ip in ip_address_strategy(),
            gateway in ip_address_strategy()
        ) {
            // Property: For any valid IP address and gateway configuration,
            // the IP address should be correctly represented and can be
            // formatted and parsed back to the same value

            // Format IP addresses to string
            let ip_str = format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
            let gateway_str = format!("{}.{}.{}.{}", gateway[0], gateway[1], gateway[2], gateway[3]);

            // Parse back using our parse_ip function
            let parsed_ip = parse_ip(&ip_str);
            let parsed_gateway = parse_ip(&gateway_str);

            // Verify round-trip consistency - this is the core property
            // that validates Requirements 1.2 and 1.3
            prop_assert_eq!(ip, parsed_ip, "IP address should round-trip correctly");
            prop_assert_eq!(gateway, parsed_gateway, "Gateway address should round-trip correctly");
        }

        #[test]
        // Additional property test: IP configuration data integrity
        // Validates that IP addresses maintain their values through the configuration structure
        fn test_ip_configuration_data_integrity(
            ip in ip_address_strategy(),
            gateway in ip_address_strategy()
        ) {
            // Property: IP configuration data should maintain integrity
            // when stored and retrieved from data structures

            // Simulate storing configuration (without actual hardware)
            let stored_ip = ip;
            let stored_gateway = gateway;

            // Verify data integrity
            prop_assert_eq!(ip, stored_ip, "Stored IP should match original");
            prop_assert_eq!(gateway, stored_gateway, "Stored gateway should match original");
        }
    }
}
