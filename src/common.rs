//! Common types and utilities for VMA socket implementations.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw::c_int;
use std::time::Duration;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

/// C-compatible VMA options structure that directly matches the C definition.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VmaOptions {
    /// Use SocketXtreme for optimized performance
    pub use_socketxtreme: bool,
    /// Optimize for low latency (may reduce throughput)
    pub optimize_for_latency: bool,
    /// Use polling for faster packet processing
    pub use_polling: bool,
    /// Number of rings to use for polling
    pub ring_count: c_int,
    /// Size of the buffer to use for packet processing
    pub buffer_size: c_int,
    /// Enable packet timestamps for latency measurements
    pub enable_timestamps: bool,
    /// Use hugepages for memory allocation
    pub use_hugepages: bool,
    /// Number of transmit buffers
    pub tx_bufs: u32,
    /// Number of receive buffers
    pub rx_bufs: u32,
    /// Prevent CPU yielding during polling
    pub disable_poll_yield: bool,
    /// Skip OS during select operations
    pub skip_os_select: bool,
    /// Keep queue pairs full for better throughput
    pub keep_qp_full: bool,
    /// CPU cores to use for VMA threads 
    pub cpu_cores: *mut c_int,
    /// Number of CPU cores in the array
    pub cpu_cores_count: c_int,
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
            use_hugepages: true,
            tx_bufs: 10000,
            rx_bufs: 10000,
            disable_poll_yield: true,
            skip_os_select: true,
            keep_qp_full: true,
            cpu_cores: std::ptr::null_mut(),
            cpu_cores_count: 0,
        }
    }
}

impl VmaOptions {
    /// Push a CPU core to the list of cores
    pub fn push_core(&mut self, core: c_int) {
        unsafe {
            // Allocate memory for the new array
            let new_count = self.cpu_cores_count + 1;
            let layout = Layout::array::<c_int>(new_count as usize).unwrap();
            let new_ptr = alloc(layout) as *mut c_int;

            if !new_ptr.is_null() {
                // Copy existing cores to the new array
                if !self.cpu_cores.is_null() {
                    ptr::copy_nonoverlapping(self.cpu_cores, new_ptr, self.cpu_cores_count as usize);
                    // Free the old array
                    let old_layout = Layout::array::<c_int>(self.cpu_cores_count as usize).unwrap();
                    dealloc(self.cpu_cores as *mut u8, old_layout);
                }

                // Add the new core to the array
                *new_ptr.add(self.cpu_cores_count as usize) = core;

                // Update the struct with the new array and count
                self.cpu_cores = new_ptr;
                self.cpu_cores_count = new_count;
            }
        }
    }

    /// # Safety
    /// This function is unsafe because it deallocates memory.
    /// Free the allocated memory for CPU cores
    pub unsafe fn free_cpu_cores(&mut self) {
        if !self.cpu_cores.is_null() {
            let layout = Layout::array::<c_int>(self.cpu_cores_count as usize).unwrap();
            dealloc(self.cpu_cores as *mut u8, layout);
            self.cpu_cores = ptr::null_mut();
            self.cpu_cores_count = 0;
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
            use_hugepages: true,
            tx_bufs: 32,
            rx_bufs: 16,
            disable_poll_yield: true,
            skip_os_select: true,
            keep_qp_full: true,
            cpu_cores: std::ptr::null_mut(),
            cpu_cores_count: 0,
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
            use_hugepages: true,
            tx_bufs: 20000, // More buffers for high throughput
            rx_bufs: 20000,
            disable_poll_yield: false, // Allow yielding for throughput
            skip_os_select: true,
            keep_qp_full: true,
            cpu_cores: std::ptr::null_mut(),
            cpu_cores_count: 0,
        }
    }
}

/// Internal representation of socket address in C format.
#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

/// Helper function to convert a Rust Duration to milliseconds for C API calls.
pub fn duration_to_ms(duration: Option<Duration>) -> c_int {
    match duration {
        Some(t) => t.as_millis() as c_int,
        None => -1, // wait indefinitely
    }
}

/// Convert a C socket address structure to a Rust SocketAddr.
pub fn sockaddr_to_rust(sockaddr: &SockAddrIn) -> SocketAddr {
    let ip = Ipv4Addr::from(u32::from_be(sockaddr.sin_addr));
    let port = u16::from_be(sockaddr.sin_port);
    SocketAddr::new(IpAddr::V4(ip), port)
}