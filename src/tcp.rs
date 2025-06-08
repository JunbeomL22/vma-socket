//! # VMA Socket
//!
//! A Rust wrapper library for the Mellanox VMA (Messaging Accelerator) library, providing 
//! high-performance networking capabilities for ultra-low latency applications.
//!
//! This library offers safe and ergonomic Rust bindings to VMA, which leverages RDMA 
//! (Remote Direct Memory Access) technology to bypass kernel overhead and deliver 
//! extremely low latency networking. The implementation includes both safe, high-level
//! Rust abstractions and lower-level FFI bindings to the C VMA library.
//!
//! ## Core Features
//!
//! - **Ultra-low latency**: Direct hardware access bypassing kernel overhead
//! - **High-level Rust API**: Safe, ergonomic wrappers for VMA functionality
//! - **Comprehensive socket types**: Support for both UDP and TCP sockets
//! - **Fine-grained control**: Detailed configuration of VMA performance parameters
//! - **Minimal overhead**: Thin wrappers that preserve VMA's performance benefits
//!
//! ## Requirements
//!
//! - Mellanox RDMA-capable network adapter (ConnectX series)
//! - Mellanox OFED drivers installed
//! - VMA library (`libvma.so`) installed
//! - Linux environment
//!
//! ## UDP Example
//!
//! ```rust,no_run
//! use vma_socket::udp::VmaUdpSocket;
//! use vma_socket::common::VmaOptions;
//!
//! fn udp_example() -> Result<(), String> {
//!     // Create socket with low-latency profile
//!     let vma_options = VmaOptions::low_latency();
//!     let mut socket = VmaUdpSocket::with_options(vma_options)?;
//!     
//!     // Server: bind to a port
//!     socket.bind("0.0.0.0", 5001)?;
//!     
//!     // Receive buffer
//!     let mut buffer = vec![0u8; 4096];
//!     
//!     // Wait for incoming packets with timeout
//!     match socket.recv_from(&mut buffer, Some(100_000_000))? {
//!         Some(packet) => {
//!             println!("Received {} bytes from {}", packet.data.len(), packet.src_addr);
//!             
//!             // Echo data back to sender
//!             let ip = packet.src_addr.ip().to_string();
//!             let port = packet.src_addr.port();
//!             socket.send_to(&packet.data, ip, port)?;
//!         },
//!         None => println!("No packet received (timeout)"),
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## TCP Example
//!
//! ```rust,no_run
//! use vma_socket::tcp::VmaTcpSocket;
//! use vma_socket::common::VmaOptions;
//!
//! fn tcp_server_example() -> Result<(), String> {
//!     // Create socket with performance optimizations
//!     let mut socket = VmaTcpSocket::with_options(VmaOptions::low_latency())?;
//!     
//!     // Bind and listen
//!     socket.bind("0.0.0.0", 5002)?;
//!     socket.listen(10)?;
//!     
//!     // Accept clients with timeout
//!     if let Some(mut client) = socket.accept(Some(1000_000_000))? {
//!         println!("Connection from {}", client.address);
//!         
//!         // Receive data
//!         let mut buffer = vec![0u8; 1024];
//!         let received = client.recv(&mut buffer, Some(100_000_000))?;
//!         
//!         // Echo back received data
//!         if received > 0 {
//!             client.send(&buffer[0..received])?;
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Running with VMA
//!
//! To run your application with VMA, use the `LD_PRELOAD` environment variable:
//!
//! ```bash
//! LD_PRELOAD=/usr/lib64/libvma.so.9.8.51 ./your_application
//! ```
//!
//! ## Module Structure
//!
//! - [`udp`]: High-performance UDP socket implementation
//! - [`tcp`]: High-performance TCP socket implementation
//! - [`common`]: Shared types and utilities used by both implementations

use crate::common::{unixnano_to_ms, sockaddr_to_rust, SockAddrIn, VmaOptions};
use std::ffi::{c_void, CString};
use std::mem;
use std::net::SocketAddr;
use std::os::raw::{c_char, c_int, c_ulonglong};

// External declarations for C functions - using VmaOptions directly
extern "C" {
    fn tcp_socket_init(socket: *mut TcpSocket, options: *const VmaOptions) -> c_int;
    fn tcp_socket_close(socket: *mut TcpSocket) -> c_int;
    fn tcp_socket_bind(socket: *mut TcpSocket, ip: *const c_char, port: u16) -> c_int;
    fn tcp_socket_listen(socket: *mut TcpSocket, backlog: c_int) -> c_int;
    fn tcp_socket_accept(socket: *mut TcpSocket, client: *mut TcpClient, timeout_ms: c_int) -> c_int;
    fn tcp_socket_connect(socket: *mut TcpSocket, ip: *const c_char, port: u16, timeout_ms: c_int) -> c_int;
    fn tcp_socket_reconnect(socket: *mut TcpSocket, timeout_ms: c_int) -> c_int;
    fn tcp_socket_is_connected(socket: *mut TcpSocket) -> bool;
    fn tcp_socket_send(socket: *mut TcpSocket, data: *const c_void, length: usize, bytes_sent: *mut usize) -> c_int;
    fn tcp_socket_send_to_client(client: *mut TcpClient, data: *const c_void, length: usize, bytes_sent: *mut usize) -> c_int;
    fn tcp_socket_recv(
        socket: *mut TcpSocket,
        buffer: *mut c_void,
        buffer_size: usize,
        timeout_ms: c_int,
        bytes_received: *mut usize,
    ) -> c_int;
    fn tcp_socket_recv_from_client(
        client: *mut TcpClient,
        buffer: *mut c_void,
        buffer_size: usize,
        timeout_ms: c_int,
        bytes_received: *mut usize,
    ) -> c_int;
    fn tcp_socket_close_client(client: *mut TcpClient) -> c_int;
    fn tcp_socket_get_stats(
        socket: *mut TcpSocket,
        rx_packets: *mut c_ulonglong,
        tx_packets: *mut c_ulonglong,
        rx_bytes: *mut c_ulonglong,
        tx_bytes: *mut c_ulonglong,
    ) -> c_int;
}

/// Connection state enumeration for TCP sockets.
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TcpConnectionState {
    /// Socket is disconnected and not in use
    Disconnected = 0,
    /// Socket is in the process of establishing a connection
    Connecting = 1,
    /// Socket is connected and ready for data transfer
    Connected = 2,
    /// Socket is in listening mode (server)
    Listening = 3,
}

/// C representation of a TCP socket.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TcpSocket {
    pub socket_fd: c_int,
    pub vma_options: VmaOptions,
    pub local_addr: SockAddrIn,
    pub remote_addr: SockAddrIn,
    pub is_bound: bool,
    pub state: TcpConnectionState,
    pub rx_packets: c_ulonglong,
    pub tx_packets: c_ulonglong,
    pub rx_bytes: c_ulonglong,
    pub tx_bytes: c_ulonglong,
    pub backlog: c_int,
}

/// C representation of a TCP client connection.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TcpClient {
    pub socket_fd: c_int,
    pub addr: SockAddrIn,
    pub rx_bytes: c_ulonglong,
    pub tx_bytes: c_ulonglong,
}

/// Result codes returned by the C TCP socket functions.
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TcpResult {
    TcpSuccess = 0,
    TcpErrorSocketCreate = -1,
    TcpErrorSocketOption = -2,
    TcpErrorBind = -3,
    TcpErrorListen = -4,
    TcpErrorAccept = -5,
    TcpErrorConnect = -6,
    TcpErrorReconnect = -7,
    TcpErrorSend = -8,
    TcpErrorRecv = -9,
    TcpErrorTimeout = -10,
    TcpErrorInvalidParam = -11,
    TcpErrorNotInitialized = -12,
    TcpErrorClosed = -13,
    TcpErrorWouldBlock = -14,
    TcpErrorAlreadyConnected = -15,
}

use std::io::{Error, ErrorKind};

impl From<TcpResult> for std::io::Error {
    fn from(tcp_result: TcpResult) -> Self {
        match tcp_result {
            TcpResult::TcpSuccess => Error::new(ErrorKind::Other, "Unexpected success"),
            TcpResult::TcpErrorSocketCreate => Error::new(ErrorKind::ConnectionRefused, "Socket creation failed"),
            TcpResult::TcpErrorSocketOption => Error::new(ErrorKind::InvalidInput, "Socket option error"),
            TcpResult::TcpErrorBind => Error::new(ErrorKind::AddrInUse, "Bind failed"),
            TcpResult::TcpErrorListen => Error::new(ErrorKind::ConnectionRefused, "Listen failed"),
            TcpResult::TcpErrorAccept => Error::new(ErrorKind::ConnectionRefused, "Accept failed"),
            TcpResult::TcpErrorConnect => Error::new(ErrorKind::ConnectionRefused, "Connect failed"),
            TcpResult::TcpErrorReconnect => Error::new(ErrorKind::ConnectionRefused, "Reconnect failed"),
            TcpResult::TcpErrorSend => Error::new(ErrorKind::BrokenPipe, "Send failed"),
            TcpResult::TcpErrorRecv => Error::new(ErrorKind::ConnectionReset, "Receive failed"),
            TcpResult::TcpErrorTimeout => Error::new(ErrorKind::TimedOut, "Operation timed out"),
            TcpResult::TcpErrorInvalidParam => Error::new(ErrorKind::InvalidInput, "Invalid parameter"),
            TcpResult::TcpErrorNotInitialized => Error::new(ErrorKind::NotConnected, "Not initialized"),
            TcpResult::TcpErrorClosed => Error::new(ErrorKind::ConnectionAborted, "Connection closed"),
            TcpResult::TcpErrorWouldBlock => Error::new(ErrorKind::WouldBlock, "Would block"),
            TcpResult::TcpErrorAlreadyConnected => Error::new(ErrorKind::AlreadyExists, "Already connected"),
        }
    }
}

/// Represents a connected client in a server context.
///
/// This structure is created when a client connects to a listening socket,
/// and provides methods for sending and receiving data to/from the client.
#[derive(Debug, Clone)]
pub struct Client {
    inner: TcpClient,
    /// The client's remote address and port
    pub address: SocketAddr,
}

impl Client {
    /// Create a new Client from a TcpClient structure.
    ///
    /// This is used internally by the accept() method.
    fn new(client: TcpClient) -> Self {
        let address = sockaddr_to_rust(&client.addr);
        Client {
            inner: client,
            address,
        }
    }
    
    /// Send data to the client.
    pub fn send(&mut self, data: &[u8]) -> Result<usize, TcpResult> {
        let mut bytes_sent: usize = 0;
        let result = unsafe {
            tcp_socket_send_to_client(
                &mut self.inner,
                data.as_ptr() as *const c_void,
                data.len(),
                &mut bytes_sent,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(bytes_sent)
    }
    
    /// Receive data from the client.
    pub fn recv(&mut self, buffer: &mut [u8], timeout_nano: Option<u64>) -> Result<usize, TcpResult> {
        let mut bytes_received: usize = 0;
        //let timeout_ms = duration_to_ms(timeout);
        let timeout_ms = unixnano_to_ms(timeout_nano);
        
        let result = unsafe {
            tcp_socket_recv_from_client(
                &mut self.inner,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                timeout_ms,
                &mut bytes_received,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(bytes_received)
    }
    
    /// Explicitly close the client connection.
    ///
    /// Note: The connection will be closed automatically when the Client is dropped.
    pub fn close(&mut self) -> Result<(), TcpResult> {
        let result = unsafe { tcp_socket_close_client(&mut self.inner) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(())
    }
}

impl Drop for Client {
    /// Automatically close the client connection when it goes out of scope.
    fn drop(&mut self) {
        if self.inner.socket_fd >= 0 {
            unsafe {
                tcp_socket_close_client(&mut self.inner);
            }
        }
    }
}

/// Low-level wrapper around the C TCP socket implementation.
/// Uses stack allocation instead of heap allocation for better performance.
#[derive(Debug, Clone)]
pub struct TcpSocketWrapper {
    socket: TcpSocket,
}

impl TcpSocketWrapper {
    /// Create a new TCP socket with the specified options.
    pub fn new(options: Option<VmaOptions>) -> Result<Self, TcpResult> {
        let mut socket = unsafe { mem::zeroed::<TcpSocket>() };
        
        let c_options = options.unwrap_or_default();
        
        let result = unsafe { 
            println!("Initializing TCP socket with options: use_socketxtreme={}, optimize_for_latency={}, ring_count={}",
                c_options.use_socketxtreme, c_options.optimize_for_latency, c_options.ring_count);
            tcp_socket_init(&mut socket, &c_options)
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            println!("TCP socket initialization failed with code: {}", result);
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(TcpSocketWrapper { socket })
    }
    
    /// Bind the socket to a local address and port.
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), TcpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { tcp_socket_bind(&mut self.socket, c_addr.as_ptr(), port) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(())
    }
    
    /// Put the socket in listening mode (server).
    pub fn listen(&mut self, backlog: i32) -> Result<(), TcpResult> {
        let result = unsafe { tcp_socket_listen(&mut self.socket, backlog) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(())
    }
    
    /// Accept a client connection (server).
    pub fn accept(&mut self, timeout_nano: Option<u64>) -> Result<Client, TcpResult> {
        let mut client = unsafe { mem::zeroed::<TcpClient>() };
        let timeout_ms = unixnano_to_ms(timeout_nano);
        
        let result = unsafe { tcp_socket_accept(&mut self.socket, &mut client, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(Client::new(client))
    }
    
    /// Connect to a server (client).
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16, timeout_nano: Option<u64>) -> Result<(), TcpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let timeout_ms = unixnano_to_ms(timeout_nano);
        
        let result = unsafe { tcp_socket_connect(&mut self.socket, c_addr.as_ptr(), port, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(())
    }
    
    /// Attempt to reconnect after a disconnection.
    pub fn reconnect(&mut self, timeout: Option<u64>) -> Result<(), TcpResult> {
        let timeout_ms = unixnano_to_ms(timeout);
        let result = unsafe { tcp_socket_reconnect(&mut self.socket, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(())
    }
    
    /// Check if the socket is currently connected.
    pub fn is_connected(&mut self) -> bool {
        unsafe { tcp_socket_is_connected(&mut self.socket) }
    }
    
    /// Send data over the connected socket.
    pub fn send(&mut self, data: &[u8]) -> Result<usize, TcpResult> {
        let mut bytes_sent: usize = 0;
        let result = unsafe {
            tcp_socket_send(
                &mut self.socket,
                data.as_ptr() as *const c_void,
                data.len(),
                &mut bytes_sent,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(bytes_sent)
    }
    
    /// Receive data from the connected socket.
    pub fn recv(&mut self, buffer: &mut [u8], timeout_nano: Option<u64>) -> Result<usize, TcpResult> {
        let mut bytes_received: usize = 0;
        let timeout_ms = unixnano_to_ms(timeout_nano);
        
        let result = unsafe {
            tcp_socket_recv(
                &mut self.socket,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                timeout_ms,
                &mut bytes_received,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok(bytes_received)
    }
    
    /// Get socket statistics.
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), TcpResult> {
        let mut rx_packets: c_ulonglong = 0;
        let mut tx_packets: c_ulonglong = 0;
        let mut rx_bytes: c_ulonglong = 0;
        let mut tx_bytes: c_ulonglong = 0;
        
        let result = unsafe {
            tcp_socket_get_stats(
                &mut self.socket as *mut _,
                &mut rx_packets,
                &mut tx_packets,
                &mut rx_bytes,
                &mut tx_bytes,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute::<i32, TcpResult>(result) });
        }
        
        Ok((rx_packets, tx_packets, rx_bytes, tx_bytes))
    }
}

impl Drop for TcpSocketWrapper {
    /// Automatically close the socket when it goes out of scope.
    fn drop(&mut self) {
        unsafe {
            tcp_socket_close(&mut self.socket);
        }
    }
}

/// High-level Rust-friendly TCP socket implementation.
#[derive(Debug, Clone)]
pub struct VmaTcpSocket {
    inner: TcpSocketWrapper,
}

impl VmaTcpSocket {
    /// Create a new TCP socket with default VMA options.
    pub fn new() -> Result<Self, std::io::Error> {
        TcpSocketWrapper::new(None)
            .map(|inner| VmaTcpSocket { inner })
            .map_err(|e| e.into())
    }
    
    /// Create a new TCP socket with custom VMA options.
    pub fn with_options(options: VmaOptions) -> Result<Self, std::io::Error> {
        TcpSocketWrapper::new(Some(options))
            .map(|inner| VmaTcpSocket { inner })
            .map_err(|e| e.into())
    }
    
    /// Bind the socket to a local address and port.
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), std::io::Error> {
        self.inner
            .bind(addr, port)
            .map_err(|e| e.into())
    }
    
    /// Put the socket in listening mode (server).
    pub fn listen(&mut self, backlog: i32) -> Result<(), std::io::Error> {
        self.inner
            .listen(backlog)
            .map_err(|e| e.into())
    }
    
    /// Accept a client connection (server).
    pub fn accept(&mut self, timeout_nano: Option<u64>) -> Result<Option<Client>, std::io::Error> {
        match self.inner.accept(timeout_nano) {
            Ok(client) => Ok(Some(client)),
            Err(TcpResult::TcpErrorTimeout) => Ok(None), // timeout is not an error
            Err(e) => Err(e.into()),
        }
    }
    
    /// Connect to a server (client).
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16, timeout: Option<u64>) -> Result<bool, std::io::Error> {
        match self.inner.connect(addr, port, timeout) {
            Ok(_) => Ok(true),
            Err(TcpResult::TcpErrorTimeout) => Ok(false), // timeout is not an error
            Err(e) => Err(e.into()),
        }
    }
    
    /// Attempt to reconnect after a disconnection.
    pub fn try_reconnect(&mut self, timeout: Option<u64>) -> Result<bool, std::io::Error> {
        match self.inner.reconnect(timeout) {
            Ok(_) => Ok(true),
            Err(TcpResult::TcpErrorTimeout) => Ok(false), // timeout is not an error
            Err(TcpResult::TcpErrorReconnect) => Ok(false), // reconnect failure is treated as a false result
            Err(e) => Err(e.into()),
        }
    }
    
    /// Check if the socket is currently connected.
    pub fn is_connected(&mut self) -> bool {
        self.inner.is_connected()
    }
    
    /// Send data over the connected socket.
    pub fn send(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        match self.inner.send(data) {
            Ok(bytes) => Ok(bytes),
            Err(TcpResult::TcpErrorWouldBlock) => Ok(0), // would block is not an error
            Err(e) => Err(e.into()),
        }
    }
    
    /// Receive data from the connected socket.
    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<u64>) -> Result<usize, std::io::Error> {
        match self.inner.recv(buffer, timeout) {
            Ok(bytes) => Ok(bytes),
            Err(TcpResult::TcpErrorTimeout) => Ok(0), // timeout is not an error
            Err(TcpResult::TcpErrorClosed) => Ok(0), // treat closed as EOF (0 bytes received)
            Err(e) => Err(e.into()),
        }
    }
    
    /// Get socket statistics.
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), std::io::Error> {
        self.inner.get_stats()
            .map_err(|e| e.into())
    }
}