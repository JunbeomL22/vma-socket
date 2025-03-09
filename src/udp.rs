//! High-performance UDP socket implementation using the VMA (Messaging Accelerator) library.
//!
//! This module provides a Rust wrapper around the VMA-accelerated UDP sockets, which offer
//! extremely low latency and high throughput networking on supported hardware.

use std::ffi::{c_void, CString};
use std::mem;
use std::net::SocketAddr;
use std::os::raw::{c_char, c_int, c_ulonglong};
use std::time::Duration;
use crate::common::{SockAddrIn, VmaOptions, duration_to_ms, sockaddr_to_rust};

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
pub struct UdpSocketWrapper {
    socket: Box<UdpSocket>,
}

impl UdpSocketWrapper {
    /// Create a new UDP socket with the specified options.
    pub fn new(options: Option<VmaOptions>) -> Result<Self, UdpResult> {
        // Clear memory for new socket
        let mut socket = Box::new(unsafe { mem::zeroed::<UdpSocket>() });
        
        // Get options - either use provided ones or defaults
        let c_options = match options {
            Some(opts) => opts,
            None => VmaOptions::default(),
        };

        // Initialize socket with options
        let result = unsafe { 
            // Print for debugging
            println!("Initializing UDP socket with options: use_socketxtreme={}, optimize_for_latency={}, ring_count={}",
                c_options.use_socketxtreme, c_options.optimize_for_latency, c_options.ring_count);
            udp_socket_init(&mut *socket, &c_options)
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
        let result = unsafe { udp_socket_bind(&mut *self.socket, c_addr.as_ptr(), port) };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, UdpResult>(result) });
        }
        
        Ok(())
    }

    /// Connect the socket to a remote address and port.
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), UdpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { udp_socket_connect(&mut *self.socket, c_addr.as_ptr(), port) };
        
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
                &mut *self.socket,
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
                &mut *self.socket,
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
    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, UdpResult> {
        let mut bytes_received: usize = 0;
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe {
            udp_socket_recv(
                &mut *self.socket,
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
    pub fn recv_from(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<Packet, UdpResult> {
        let mut packet = unsafe { mem::zeroed::<UdpPacket>() };
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe {
            udp_socket_recvfrom(
                &mut *self.socket,
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
                &mut *self.socket as *mut _,
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
            udp_socket_close(&mut *self.socket);
        }
    }
}

// High-level Rust-friendly API for UDP sockets
pub struct VmaUdpSocket {
    inner: UdpSocketWrapper,
}

impl VmaUdpSocket {
    /// Create a new UDP socket with default VMA options.
    pub fn new() -> Result<Self, String> {
        UdpSocketWrapper::new(None)
            .map(|inner| VmaUdpSocket { inner })
            .map_err(|e| format!("Failed to create socket: {:?}", e))
    }

    /// Create a new UDP socket with custom VMA options.
    pub fn with_options(options: VmaOptions) -> Result<Self, String> {
        UdpSocketWrapper::new(Some(options))
            .map(|inner| VmaUdpSocket { inner })
            .map_err(|e| format!("Failed to create socket with options: {:?}", e))
    }

    /// Bind the socket to a local address and port.
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), String> {
        self.inner
            .bind(addr, port)
            .map_err(|e| format!("Failed to bind: {:?}", e))
    }

    /// Connect the socket to a remote address and port.
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), String> {
        self.inner
            .connect(addr, port)
            .map_err(|e| format!("Failed to connect: {:?}", e))
    }

    /// Send data to the connected remote address.
    pub fn send(&mut self, data: &[u8]) -> Result<usize, String> {
        self.inner
            .send(data)
            .map_err(|e| format!("Failed to send: {:?}", e))
    }

    /// Send data to a specified address and port.
    pub fn send_to<A: Into<String>>(&mut self, data: &[u8], addr: A, port: u16) -> Result<usize, String> {
        self.inner
            .send_to(data, addr, port)
            .map_err(|e| format!("Failed to send to address: {:?}", e))
    }

    /// Receive data from the connected remote address.
    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, String> {
        match self.inner.recv(buffer, timeout) {
            Ok(bytes) => Ok(bytes),
            Err(UdpResult::UdpErrorTimeout) => Ok(0), // timeout is not an error
            Err(e) => Err(format!("Failed to receive: {:?}", e)),
        }
    }

    /// Receive data and source address information.
    pub fn recv_from(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<Option<Packet>, String> {
        match self.inner.recv_from(buffer, timeout) {
            Ok(packet) => Ok(Some(packet)),
            Err(UdpResult::UdpErrorTimeout) => Ok(None), // timeout is not an error
            Err(e) => Err(format!("Failed to receive from: {:?}", e)),
        }
    }

    /// Get socket statistics.
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), String> {
        self.inner
            .get_stats()
            .map_err(|e| format!("Failed to get stats: {:?}", e))
    }
}