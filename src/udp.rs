//! UDP socket implementation accelerated by the VMA (Messaging Accelerator) library.
//!
//! This module provides high-performance UDP sockets designed for ultra-low latency 
//! networking applications. It leverages the Mellanox VMA library to bypass kernel 
//! overhead and achieve microsecond-level latencies on supported RDMA hardware.
//!
//! The implementation consists of both high-level, safe Rust abstractions ([`VmaUdpSocket`])
//! and lower-level FFI bindings to the C VMA library ([`UdpSocketWrapper`]).
//!
//! # Features
//!
//! - Direct hardware access for minimal latency (kernel bypass)
//! - Zero-copy optimizations where possible
//! - Configurable latency/throughput profiles
//! - Support for timestamping on packet reception
//! - Socket polling modes for lowest possible latency
//! - Comprehensive performance tuning options
//!
//! # Performance Considerations
//!
//! For best performance:
//!
//! - Use `VmaOptions::low_latency()` for latency-sensitive applications
//! - Enable polling mode for lowest latencies (higher CPU usage)
//! - Consider using SocketXtreme mode for maximum performance
//! - Set appropriate CPU affinity for networking threads
//! - Use direct connection (via `connect()`) when sending to a single target
//!
//! # Examples
//!
//! ## Creating a UDP server
//!
//! ```rust,no_run
//! use vma_socket::udp::VmaUdpSocket;
//! use vma_socket::common::VmaOptions;
//!
//! // Create socket with low latency optimizations
//! let options = VmaOptions::low_latency();
//! let mut socket = VmaUdpSocket::with_options(options).unwrap();
//!
//! // Bind to address and port
//! socket.bind("0.0.0.0", 5001).unwrap();
//!
//! // Receive buffer
//! let mut buffer = vec![0u8; 4096];
//!
//! // Receive data with timeout
//! match socket.recv_from(&mut buffer, Some(100_000_000)) { // 100ms timeout
//!     Some(packet) => {
//!         println!("Received {} bytes from {}", packet.data.len(), packet.src_addr);
//!         println!("Packet timestamp: {} ns", packet.timestamp);
//!     },
//!     None => println!("No packet received (timeout)"),
//! }
//! ```
//!
//! ## Creating a UDP client
//!
//! ```rust,no_run
//! use vma_socket::udp::VmaUdpSocket;
//! use vma_socket::common::VmaOptions;
//!
//! // Create socket with throughput optimizations
//! let options = VmaOptions::high_throughput();
//! let mut socket = VmaUdpSocket::with_options(options).unwrap();
//!
//! // Connect to target (sets default destination)
//! socket.connect("192.168.1.100", 5001).unwrap();
//!
//! // Send data
//! let data = b"Hello VMA!";
//! let bytes_sent = socket.send(data).unwrap();
//! println!("Sent {} bytes", bytes_sent);
//!
//! // Or send to a specific target without prior connect()
//! socket.send_to(data, "192.168.1.101", 5002).unwrap();
//! ```
//!
//! ## Performance statistics
//!
//! ```rust,no_run
//! use vma_socket::udp::VmaUdpSocket;
//!
//! let mut socket = VmaUdpSocket::new().unwrap();
//! // ... use socket ...
//!
//! // Get performance statistics
//! let (rx_packets, tx_packets, rx_bytes, tx_bytes) = socket.get_stats().unwrap();
//! println!("Stats: RX {}p/{}b, TX {}p/{}b", 
//!          rx_packets, rx_bytes, tx_packets, tx_bytes);
//! ```

use std::ffi::{c_void, CString};
use std::mem;
use std::net::SocketAddr;
use std::os::raw::{c_char, c_int, c_ulonglong};
use crate::common::{SockAddrIn, VmaOptions, unixnano_to_ms, sockaddr_to_rust};

/// C representation of a UDP socket.
#[repr(C)]
pub struct UdpSocket {
    pub socket_fd: c_int,
    pub vma_options: VmaOptions,
    pub local_addr: SockAddrIn,
    pub remote_addr: SockAddrIn,
    pub is_bound: bool,
    pub is_connected: bool,
    pub rx_packets: c_ulonglong,
    pub tx_packets: c_ulonglong,
    pub rx_bytes: c_ulonglong,
    pub tx_bytes: c_ulonglong,
}

/// C representation of a UDP packet.
#[repr(C)]
pub struct UdpPacket {
    pub data: *mut c_void,
    pub length: usize,
    pub src_addr: SockAddrIn,
    pub timestamp: c_ulonglong,
}

/// Result codes returned by the C UDP socket functions.
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum UdpResult {
    UdpSuccess = 0,
    UdpErrorSocketCreate = -1,
    UdpErrorSocketOption = -2,
    UdpErrorBind = -3,
    UdpErrorConnect = -4,
    UdpErrorSend = -5,
    UdpErrorRecv = -6,
    UdpErrorTimeout = -7,
    UdpErrorInvalidParam = -8,
    UdpErrorNotInitialized = -9,
    UdpErrorClosed = -10,
}

use std::io::{Error, ErrorKind};
impl From<UdpResult> for std::io::Error {
    fn from(udp_result: UdpResult) -> Self {
        match udp_result {
            UdpResult::UdpSuccess => Error::new(ErrorKind::Other, "Unexpected success"),
            UdpResult::UdpErrorSocketCreate => Error::new(ErrorKind::ConnectionRefused, "Socket creation failed"),
            UdpResult::UdpErrorSocketOption => Error::new(ErrorKind::InvalidInput, "Socket option error"),
            UdpResult::UdpErrorBind => Error::new(ErrorKind::AddrInUse, "Bind failed"),
            UdpResult::UdpErrorConnect => Error::new(ErrorKind::ConnectionRefused, "Connect failed"),
            UdpResult::UdpErrorSend => Error::new(ErrorKind::BrokenPipe, "Send failed"),
            UdpResult::UdpErrorRecv => Error::new(ErrorKind::ConnectionReset, "Receive failed"),
            UdpResult::UdpErrorTimeout => Error::new(ErrorKind::TimedOut, "Operation timed out"),
            UdpResult::UdpErrorInvalidParam => Error::new(ErrorKind::InvalidInput, "Invalid parameter"),
            UdpResult::UdpErrorNotInitialized => Error::new(ErrorKind::NotConnected, "Not initialized"),
            UdpResult::UdpErrorClosed => Error::new(ErrorKind::ConnectionAborted, "Socket closed"),
        }
    }
}

// External declarations for C functions - using VmaOptions directly
extern "C" {
    fn udp_socket_init(socket: *mut UdpSocket, options: *const VmaOptions) -> c_int;
    fn udp_socket_close(socket: *mut UdpSocket) -> c_int;
    fn udp_socket_bind(socket: *mut UdpSocket, ip: *const c_char, port: u16) -> c_int;
    fn udp_socket_connect(socket: *mut UdpSocket, ip: *const c_char, port: u16) -> c_int;
    fn udp_socket_send(socket: *mut UdpSocket, data: *const c_void, length: usize, bytes_sent: *mut usize) -> c_int;
    fn udp_socket_sendto(
        socket: *mut UdpSocket,
        data: *const c_void,
        length: usize,
        ip: *const c_char,
        port: u16,
        bytes_sent: *mut usize,
    ) -> c_int;
    fn udp_socket_recv(
        socket: *mut UdpSocket,
        buffer: *mut c_void,
        buffer_size: usize,
        timeout_ms: c_int,
        bytes_received: *mut usize,
    ) -> c_int;
    fn udp_socket_recvfrom(
        socket: *mut UdpSocket,
        packet: *mut UdpPacket,
        buffer: *mut c_void,
        buffer_size: usize,
        timeout_ms: c_int,
    ) -> c_int;
    fn udp_socket_get_stats(
        socket: *mut UdpSocket,
        rx_packets: *mut c_ulonglong,
        tx_packets: *mut c_ulonglong,
        rx_bytes: *mut c_ulonglong,
        tx_bytes: *mut c_ulonglong,
    ) -> c_int;
}

/// A received UDP packet with associated metadata.
#[derive(Debug)]
pub struct Packet {
    /// The packet payload data.
    pub data: Vec<u8>,
    
    /// The source address from which the packet was received.
    pub src_addr: SocketAddr,
    
    /// Hardware timestamp (if available) in nanoseconds since the epoch.
    pub timestamp: u64,
}

/// Low-level wrapper around the C UDP socket implementation.
/// Uses stack allocation instead of heap allocation for better performance.
pub struct UdpSocketWrapper {
    socket: UdpSocket,
}

impl UdpSocketWrapper {
    /// Create a new UDP socket with the specified options.
    pub fn new(options: Option<VmaOptions>) -> Result<Self, UdpResult> {
        // Clear memory for new socket
        let mut socket = unsafe { mem::zeroed::<UdpSocket>() };
        
        // Get options - either use provided ones or defaults
        let c_options = options.unwrap_or_default();

        // Initialize socket with options
        let result = unsafe { 
            // Print for debugging
            println!("Initializing UDP socket with options: use_socketxtreme={}, optimize_for_latency={}, ring_count={}",
                c_options.use_socketxtreme, c_options.optimize_for_latency, c_options.ring_count);
            udp_socket_init(&mut socket, &c_options)
        };
        
        if result != UdpResult::UdpSuccess as i32 {
            println!("UDP socket initialization failed with code: {}", result);
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok(UdpSocketWrapper { socket })
    }

    /// Bind the socket to a local address and port.
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), UdpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { udp_socket_bind(&mut self.socket, c_addr.as_ptr(), port) };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok(())
    }

    /// Connect the socket to a remote address and port.
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), UdpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { udp_socket_connect(&mut self.socket, c_addr.as_ptr(), port) };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok(())
    }

    /// Send data to the connected remote address.
    pub fn send(&mut self, data: &[u8]) -> Result<usize, UdpResult> {
        let mut bytes_sent: usize = 0;
        let result = unsafe {
            udp_socket_send(
                &mut self.socket,
                data.as_ptr() as *const c_void,
                data.len(),
                &mut bytes_sent,
            )
        };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok(bytes_sent)
    }

    /// Send data to a specified address and port.
    pub fn send_to<A: Into<String>>(&mut self, data: &[u8], addr: A, port: u16) -> Result<usize, UdpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let mut bytes_sent: usize = 0;
        
        let result = unsafe {
            udp_socket_sendto(
                &mut self.socket,
                data.as_ptr() as *const c_void,
                data.len(),
                c_addr.as_ptr(),
                port,
                &mut bytes_sent,
            )
        };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok(bytes_sent)
    }

    /// Receive data from the connected remote address.
    pub fn recv(&mut self, buffer: &mut [u8], timeout_nano: Option<u64>) -> Result<usize, UdpResult> {
        let mut bytes_received: usize = 0;
        let timeout_ms = unixnano_to_ms(timeout_nano);
        
        let result = unsafe {
            udp_socket_recv(
                &mut self.socket,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                timeout_ms,
                &mut bytes_received,
            )
        };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok(bytes_received)
    }

    /// Receive data and source address information.
    pub fn recv_from(&mut self, buffer: &mut [u8], timeout_nano: Option<u64>) -> Result<Packet, UdpResult> {
        let mut packet = unsafe { mem::zeroed::<UdpPacket>() };
        let timeout_ms = unixnano_to_ms(timeout_nano);
        
        let result = unsafe {
            udp_socket_recvfrom(
                &mut self.socket,
                &mut packet,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                timeout_ms,
            )
        };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        // Convert sockaddr to Rust SocketAddr
        let src_addr = sockaddr_to_rust(&packet.src_addr);
        
        // Copy data
        let data = unsafe { std::slice::from_raw_parts(packet.data as *const u8, packet.length) }.to_vec();
        
        Ok(Packet {
            data,
            src_addr,
            timestamp: packet.timestamp,
        })
    }

    /// Get socket statistics.
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), UdpResult> {
        let mut rx_packets: c_ulonglong = 0;
        let mut tx_packets: c_ulonglong = 0;
        let mut rx_bytes: c_ulonglong = 0;
        let mut tx_bytes: c_ulonglong = 0;
        
        let result = unsafe {
            udp_socket_get_stats(
                &mut self.socket as *mut _,
                &mut rx_packets,
                &mut tx_packets,
                &mut rx_bytes,
                &mut tx_bytes,
            )
        };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok((rx_packets, tx_packets, rx_bytes, tx_bytes))
    }
}

impl Drop for UdpSocketWrapper {
    fn drop(&mut self) {
        unsafe {
            udp_socket_close(&mut self.socket);
        }
    }
}

// High-level Rust-friendly API for UDP sockets
pub struct VmaUdpSocket {
    inner: UdpSocketWrapper,
}
impl VmaUdpSocket {
    /// Create a new UDP socket with default VMA options.
    pub fn new() -> Result<Self, std::io::Error> {
        UdpSocketWrapper::new(None)
            .map(|inner| VmaUdpSocket { inner })
            .map_err(|e| e.into())
    }

    /// Create a new UDP socket with custom VMA options.
    pub fn with_options(options: VmaOptions) -> Result<Self, std::io::Error> {
        UdpSocketWrapper::new(Some(options))
            .map(|inner| VmaUdpSocket { inner })
            .map_err(|e| e.into())
    }

    /// Bind the socket to a local address and port.
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), std::io::Error> {
        self.inner
            .bind(addr, port)
            .map_err(|e| e.into())
    }

    /// Connect the socket to a remote address and port.
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), std::io::Error> {
        self.inner
            .connect(addr, port)
            .map_err(|e| e.into())
    }

    /// Send data to the connected remote address.
    pub fn send(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        self.inner
            .send(data)
            .map_err(|e| e.into())
    }

    /// Send data to a specified address and port.
    pub fn send_to<A: Into<String>>(&mut self, data: &[u8], addr: A, port: u16) -> Result<usize, std::io::Error> {
        self.inner
            .send_to(data, addr, port)
            .map_err(|e| e.into())
    }

    /// Receive data from the connected remote address.
    pub fn recv(&mut self, buffer: &mut [u8], timeout_nano: Option<u64>) -> Result<usize, std::io::Error> {
        match self.inner.recv(buffer, timeout_nano) {
            Ok(bytes) => Ok(bytes),
            Err(UdpResult::UdpErrorTimeout) => Ok(0), // timeout is not an error
            Err(e) => Err(e.into()),
        }
    }

    /// Receive data and source address information.
    pub fn recv_from(&mut self, buffer: &mut [u8], timeout_nano: Option<u64>) -> Result<Option<Packet>, std::io::Error> {
        match self.inner.recv_from(buffer, timeout_nano) {
            Ok(packet) => Ok(Some(packet)),
            Err(UdpResult::UdpErrorTimeout) => Ok(None), // timeout is not an error
            Err(e) => Err(e.into()),
        }
    }

    /// Get socket statistics.
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), std::io::Error> {
        self.inner
            .get_stats()
            .map_err(|e| e.into())
    }
}
