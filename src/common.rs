//! Common types and utilities for VMA socket implementations.
//!
//! This module contains shared structures and helper functions used by both
//! the TCP and UDP implementations of VMA sockets.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw::c_int;
use std::time::Duration;

/// Enhanced VMA options structure with additional performance configuration.
#[derive(Debug, Clone)]
pub struct VmaOptions {
    /// Use SocketXtreme for optimized performance
    pub use_socketxtreme: bool,
    /// Optimize for low latency (may reduce throughput)
    pub optimize_for_latency: bool,
    /// Use polling for faster packet processing
    pub use_polling: bool,
    /// Number of rings to use for polling (affects VMA_RING_ALLOCATION_LOGIC_RX)
    pub ring_count: i32,
    /// Size of the buffer to use for packet processing (affects VMA_PACKET_SIZE)
    pub buffer_size: i32,
    /// Enable packet timestamps for latency measurements (affects VMA_PACKET_TIMESTAMP)
    pub enable_timestamps: bool,
    /// CPU cores to use for VMA threads (affects VMA_THREAD_AFFINITY_ID)
    pub cpu_cores: Option<Vec<usize>>,
    
    /// Use hugepages for memory allocation (affects VMA_MEMORY_ALLOCATION_TYPE)
    pub use_hugepages: bool,
    
    /// Number of transmit buffers (affects VMA_TX_BUFS)
    pub tx_bufs: u32,
    
    /// Number of receive buffers (affects VMA_RX_BUFS)
    pub rx_bufs: u32,
    
    /// Prevent CPU yielding during polling (affects VMA_RX_POLL_YIELD)
    pub disable_poll_yield: bool,
    
    /// Skip OS during select operations (affects VMA_SELECT_SKIP_OS)
    pub skip_os_select: bool,
    
    /// Keep queue pairs full for better throughput (affects VMA_CQ_KEEP_QP_FULL)
    pub keep_qp_full: bool,
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
            cpu_cores: None,
            use_hugepages: true,
            tx_bufs: 10000,
            rx_bufs: 10000,
            disable_poll_yield: true,
            skip_os_select: true,
            keep_qp_full: true,
        }
    }
}

impl VmaOptions {
    /// Apply all VMA environment variables based on these options
    pub fn apply_environment_variables(&self) {
        use std::env;
        
        // Original VMA settings
        if self.use_socketxtreme {
            env::set_var("VMA_SOCKETXTREME", "1");
        }
        
        if self.optimize_for_latency {
            env::set_var("VMA_SPEC", "latency");
        }
        
        if self.use_polling {
            env::set_var("VMA_RX_POLL", "1");
            env::set_var("VMA_SELECT_POLL", "1");
            
            // Additional polling optimizations
            if self.disable_poll_yield {
                env::set_var("VMA_RX_POLL_YIELD", "0");
            }
            
            if self.skip_os_select {
                env::set_var("VMA_SELECT_SKIP_OS", "1");
            }
        }
        
        if self.ring_count > 0 {
            env::set_var("VMA_RING_ALLOCATION_LOGIC_RX", self.ring_count.to_string());
        }
        
        // SocketXtreme optimization
        if self.use_socketxtreme {
            env::set_var("VMA_RING_ALLOCATION_LOGIC_TX", "0");
            env::set_var("VMA_THREAD_MODE", "1");
            
            if self.keep_qp_full {
                env::set_var("VMA_CQ_KEEP_QP_FULL", "1");
            }
        } else {
            // Multi-threaded mode when not using SocketXtreme
            env::set_var("VMA_THREAD_MODE", "3");
        }
        
        // Apply CPU affinity if specified
        if let Some(cores) = &self.cpu_cores {
            if !cores.is_empty() {
                env::set_var("VMA_THREAD_AFFINITY", "1");
                let cores_str = cores.iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                env::set_var("VMA_THREAD_AFFINITY_ID", cores_str);
            }
        }
        
        // Apply hugepages setting
        if self.use_hugepages {
            env::set_var("VMA_MEMORY_ALLOCATION_TYPE", "2");
        }
        
        // Apply buffer count settings
        if self.tx_bufs > 0 {
            env::set_var("VMA_TX_BUFS", self.tx_bufs.to_string());
        }
        
        if self.rx_bufs > 0 {
            env::set_var("VMA_RX_BUFS", self.rx_bufs.to_string());
        }
    }
    
    /// Create options optimized for ultra-low latency
    pub fn low_latency() -> Self {
        VmaOptions {
            use_socketxtreme: true,
            optimize_for_latency: true,
            use_polling: true,
            ring_count: 1, // Single ring for lower latency
            buffer_size: 8192, // Smaller buffers
            enable_timestamps: true,
            cpu_cores: None, // Set this based on your system
            use_hugepages: true,
            tx_bufs: 32,
            rx_bufs: 16,
            disable_poll_yield: true,
            skip_os_select: true,
            keep_qp_full: true,
        }
    }
    
    /// Create options optimized for high throughput
    pub fn high_throughput() -> Self {
        VmaOptions {
            use_socketxtreme: true,
            optimize_for_latency: false, // Optimize for throughput instead
            use_polling: true,
            ring_count: 4, // Multiple rings for throughput
            buffer_size: 65536, // Larger buffers (64KB)
            enable_timestamps: false, // Disable timestamps for throughput
            cpu_cores: None, // Set this based on your system
            use_hugepages: true,
            tx_bufs: 20000, // More buffers for high throughput
            rx_bufs: 20000,
            disable_poll_yield: false, // Allow yielding for throughput
            skip_os_select: true,
            keep_qp_full: true,
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