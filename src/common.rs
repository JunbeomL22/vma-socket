//! Common types and utilities for VMA socket implementations.
//!
//! This module contains shared structures and helper functions used by both
//! the TCP and UDP implementations of VMA sockets.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw::c_int;
use std::time::Duration;

/// Common VMA options structure for both TCP and UDP socket types.
///
/// This structure allows configuration of VMA-specific options that control
/// the performance and behavior of the underlying socket implementation.
#[derive(Debug, Clone)]
pub struct VmaOptions {
    /// Enable SocketXtreme mode for maximum performance in single-threaded applications.
    ///
    /// SocketXtreme bypasses the kernel for even lower latency communication but is
    /// primarily designed for single-threaded use cases.
    pub use_socketxtreme: bool,

    /// Optimize the socket for low latency rather than high throughput.
    ///
    /// When set to true, the socket implementation prioritizes minimizing latency
    /// potentially at the expense of maximum throughput.
    pub optimize_for_latency: bool,

    /// Use polling mode instead of event-based notification.
    ///
    /// Polling mode can provide lower latency but at the cost of higher CPU usage,
    /// as it continuously checks for new data rather than waiting for notifications.
    pub use_polling: bool,

    /// Number of VMA rings to allocate.
    ///
    /// For multi-threaded applications, increasing this value can improve performance
    /// by allowing multiple threads to process network operations concurrently.
    pub ring_count: i32,

    /// Size of send and receive buffers.
    ///
    /// Setting an appropriate buffer size for your application can improve performance
    /// by reducing the number of system calls needed for larger data transfers.
    pub buffer_size: i32,

    /// Enable hardware timestamps for received packets.
    ///
    /// When supported by the network adapter, hardware timestamps provide more
    /// accurate timing information about when packets were received.
    pub enable_timestamps: bool,
}

impl Default for VmaOptions {
    /// Creates a default configuration with reasonable settings for most applications.
    ///
    /// The default configuration enables SocketXtreme, optimizes for latency, enables
    /// polling mode, uses 4 rings, sets a 64KB buffer size, and enables timestamps.
    fn default() -> Self {
        VmaOptions {
            use_socketxtreme: true,
            optimize_for_latency: true,
            use_polling: true,
            ring_count: 4,
            buffer_size: 65536, // 64KB
            enable_timestamps: true,
        }
    }
}

/// Internal representation of socket address in C format.
///
/// This structure matches the layout of the C sockaddr_in structure for compatibility
/// with the underlying VMA library API.
#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

/// Helper function to convert a Rust Duration to milliseconds for C API calls.
///
/// Returns -1 for None (indicating an indefinite wait) or the duration in milliseconds.
///
/// # Parameters
///
/// * `duration` - Optional Duration representing a timeout period
pub fn duration_to_ms(duration: Option<Duration>) -> c_int {
    match duration {
        Some(t) => t.as_millis() as c_int,
        None => -1, // wait indefinitely
    }
}

/// Convert a C socket address structure to a Rust SocketAddr.
///
/// # Parameters
///
/// * `sockaddr` - Reference to a C-compatible SockAddrIn structure
///
/// # Returns
///
/// A Rust SocketAddr representing the same address
pub fn sockaddr_to_rust(sockaddr: &SockAddrIn) -> SocketAddr {
    let ip = Ipv4Addr::from(u32::from_be(sockaddr.sin_addr));
    let port = u16::from_be(sockaddr.sin_port);
    SocketAddr::new(IpAddr::V4(ip), port)
}