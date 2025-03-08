// Common socket types and utilities

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw::c_int;
use std::time::Duration;

// Common VMA options structure for both TCP and UDP
pub struct VmaOptions {
    pub use_socketxtreme: bool,
    pub optimize_for_latency: bool,
    pub use_polling: bool,
    pub ring_count: i32,
    pub buffer_size: i32,
    pub enable_timestamps: bool,
}

impl Default for VmaOptions {
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

// Socket address C representation
#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

// Helper functions
pub fn duration_to_ms(duration: Option<Duration>) -> c_int {
    match duration {
        Some(t) => t.as_millis() as c_int,
        None => -1, // wait indefinitely
    }
}

// Socket address conversion
pub fn sockaddr_to_rust(sockaddr: &SockAddrIn) -> SocketAddr {
    let ip = Ipv4Addr::from(u32::from_be(sockaddr.sin_addr));
    let port = u16::from_be(sockaddr.sin_port);
    SocketAddr::new(IpAddr::V4(ip), port)
}