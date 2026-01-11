//! UDP Communication Library
//!
//! This library provides configuration and WiFi management functionality
//! for UDP communication on ESP32 series chips.
//!
//! This module is used for testing purposes and exposes the internal
//! modules for unit and property-based testing.

// Only use no_std when not testing (tests run on host with std)
#![cfg_attr(not(test), no_std)]

// Config module doesn't depend on embedded hardware
pub mod config;

#[cfg(test)]
mod udp_server_tests;

#[cfg(test)]
mod udp_client_tests;

