//! High-performance TCP socket implementation using the VMA (Messaging Accelerator) library.
//!
//! This module provides a Rust wrapper around the VMA-accelerated TCP sockets, which offer
//! extremely low latency and high throughput networking on supported hardware. The implementation
//! includes both low-level FFI bindings to the C VMA library and a high-level, safe Rust API.
//!
//! # Example
//!
//! ```rust
//! use std::time::Duration;
//! use vma_socket::tcp::{VmaTcpSocket};
//! use vma_socket::common::VmaOptions;
//!
//! // Create a socket with default VMA options
//! let mut socket = VmaTcpSocket::new().unwrap();
//!
//! // Server mode
//! socket.bind("0.0.0.0", 5002).unwrap();
//! socket.listen(10).unwrap();
//!
//! match socket.accept(Some(Duration::from_secs(1))) {
//!     Ok(Some(client)) => {
//!         println!("Connection from {}", client.address);
//!         // Handle client connection...
//!     },
//!     Ok(None) => println!("No connections within timeout"),
//!     Err(e) => println!("Error: {}", e),
//! }
//!
//! // Client mode
//! let mut socket = VmaTcpSocket::new().unwrap();
//! match socket.connect("127.0.0.1", 5002, Some(Duration::from_secs(5))) {
//!     Ok(true) => println!("Connected!"),
//!     Ok(false) => println!("Connection timeout"),
//!     Err(e) => println!("Error: {}", e),
//! }
//! ```
use crate::common::{SockAddrIn, VmaOptions, duration_to_ms, sockaddr_to_rust};
use std::ffi::{c_void, CString};
use std::mem;
use std::net::SocketAddr;
use std::os::raw::{c_char, c_int, c_ulonglong};
use std::time::Duration;


// External declarations for C functions
extern "C" {
    fn tcp_socket_init(socket: *mut TcpSocket, options: *const TcpVmaOptions) -> c_int;
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

/// TCP socket options specifically for the VMA library.
///
/// This structure is used in the C FFI layer and corresponds to the
/// `tcp_vma_options_t` struct in the C code.
#[repr(C)]
pub struct TcpVmaOptions {
    pub use_socketxtreme: bool,
    pub optimize_for_latency: bool,
    pub use_polling: bool,
    pub ring_count: c_int,
    pub buffer_size: c_int,
    pub enable_timestamps: bool,
}

impl From<VmaOptions> for TcpVmaOptions {
    fn from(opts: VmaOptions) -> Self {
        TcpVmaOptions {
            use_socketxtreme: opts.use_socketxtreme,
            optimize_for_latency: opts.optimize_for_latency,
            use_polling: opts.use_polling,
            ring_count: opts.ring_count,
            buffer_size: opts.buffer_size,
            enable_timestamps: opts.enable_timestamps,
        }
    }
}

/// C representation of a TCP socket.
#[repr(C)]
pub struct TcpSocket {
    pub socket_fd: c_int,
    pub vma_options: TcpVmaOptions,
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
pub struct TcpClient {
    pub socket_fd: c_int,
    pub addr: SockAddrIn,
    pub rx_bytes: c_ulonglong,
    pub tx_bytes: c_ulonglong,
}

/// Result codes returned by the C TCP socket functions.
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
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

/// Represents a connected client in a server context.
///
/// This structure is created when a client connects to a listening socket,
/// and provides methods for sending and receiving data to/from the client.
pub struct Client {
    inner: Box<TcpClient>,
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
            inner: Box::new(client),
            address,
        }
    }
    
    /// Send data to the client.
    ///
    /// # Parameters
    ///
    /// * `data` - The data to send
    ///
    /// # Returns
    ///
    /// A Result containing either the number of bytes sent or an error code
    pub fn send(&mut self, data: &[u8]) -> Result<usize, TcpResult> {
        let mut bytes_sent: usize = 0;
        let result = unsafe {
            tcp_socket_send_to_client(
                &mut *self.inner,
                data.as_ptr() as *const c_void,
                data.len(),
                &mut bytes_sent,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(bytes_sent)
    }
    
    /// Receive data from the client.
    ///
    /// # Parameters
    ///
    /// * `buffer` - Buffer to store the received data
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either the number of bytes received or an error code
    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, TcpResult> {
        let mut bytes_received: usize = 0;
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe {
            tcp_socket_recv_from_client(
                &mut *self.inner,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                timeout_ms,
                &mut bytes_received,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(bytes_received)
    }
    
    /// Explicitly close the client connection.
    ///
    /// Note: The connection will be closed automatically when the Client is dropped.
    ///
    /// # Returns
    ///
    /// A Result containing either success or an error code
    pub fn close(&mut self) -> Result<(), TcpResult> {
        let result = unsafe { tcp_socket_close_client(&mut *self.inner) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
}

impl Drop for Client {
    /// Automatically close the client connection when it goes out of scope.
    fn drop(&mut self) {
        if self.inner.socket_fd >= 0 {
            unsafe {
                tcp_socket_close_client(&mut *self.inner);
            }
        }
    }
}
/// Low-level wrapper around the C TCP socket implementation.
///
/// This structure provides a direct wrapper around the C TCP socket functions,
/// handling memory management and FFI conversions, but maintaining a close mapping
/// to the original API.
pub struct TcpSocketWrapper {
    socket: Box<TcpSocket>,
}

impl TcpSocketWrapper {
    /// Create a new TCP socket with the specified options.
    ///
    /// # Parameters
    ///
    /// * `options` - Optional VMA configuration options
    ///
    /// # Returns
    ///
    /// A Result containing either the new socket or an error code
    pub fn new(options: Option<VmaOptions>) -> Result<Self, TcpResult> {
        let mut socket = Box::new(unsafe { mem::zeroed::<TcpSocket>() });
        
        let c_options: TcpVmaOptions = match options {
            Some(opts) => opts.into(),
            None => VmaOptions::default().into(),
        };
        
        let result = unsafe { tcp_socket_init(&mut *socket, &c_options) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(TcpSocketWrapper { socket })
    }
    
    /// Bind the socket to a local address and port.
    ///
    /// # Parameters
    ///
    /// * `addr` - IP address to bind to
    /// * `port` - Port number to bind to
    ///
    /// # Returns
    ///
    /// A Result containing either success or an error code
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), TcpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { tcp_socket_bind(&mut *self.socket, c_addr.as_ptr(), port) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    /// Put the socket in listening mode (server).
    ///
    /// # Parameters
    ///
    /// * `backlog` - Maximum number of pending connections to queue
    ///
    /// # Returns
    ///
    /// A Result containing either success or an error code
    pub fn listen(&mut self, backlog: i32) -> Result<(), TcpResult> {
        let result = unsafe { tcp_socket_listen(&mut *self.socket, backlog) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    /// Accept a client connection (server).
    ///
    /// # Parameters
    ///
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either a Client or an error code
    pub fn accept(&mut self, timeout: Option<Duration>) -> Result<Client, TcpResult> {
        let mut client = unsafe { mem::zeroed::<TcpClient>() };
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe { tcp_socket_accept(&mut *self.socket, &mut client, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(Client::new(client))
    }
    
    /// Connect to a server (client).
    ///
    /// # Parameters
    ///
    /// * `addr` - Server IP address
    /// * `port` - Server port number
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either success or an error code
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16, timeout: Option<Duration>) -> Result<(), TcpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe { tcp_socket_connect(&mut *self.socket, c_addr.as_ptr(), port, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    /// Attempt to reconnect after a disconnection.
    ///
    /// # Parameters
    ///
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either success or an error code
    pub fn reconnect(&mut self, timeout: Option<Duration>) -> Result<(), TcpResult> {
        let timeout_ms = duration_to_ms(timeout);
        let result = unsafe { tcp_socket_reconnect(&mut *self.socket, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    /// Check if the socket is currently connected.
    ///
    /// # Returns
    ///
    /// True if connected, false otherwise
    pub fn is_connected(&mut self) -> bool {
        unsafe { tcp_socket_is_connected(&mut *self.socket) }
    }
    
    /// Send data over the connected socket.
    ///
    /// # Parameters
    ///
    /// * `data` - The data to send
    ///
    /// # Returns
    ///
    /// A Result containing either the number of bytes sent or an error code
    pub fn send(&mut self, data: &[u8]) -> Result<usize, TcpResult> {
        let mut bytes_sent: usize = 0;
        let result = unsafe {
            tcp_socket_send(
                &mut *self.socket,
                data.as_ptr() as *const c_void,
                data.len(),
                &mut bytes_sent,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(bytes_sent)
    }
    
    /// Receive data from the connected socket.
    ///
    /// # Parameters
    ///
    /// * `buffer` - Buffer to store the received data
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either the number of bytes received or an error code
    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, TcpResult> {
        let mut bytes_received: usize = 0;
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe {
            tcp_socket_recv(
                &mut *self.socket,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                timeout_ms,
                &mut bytes_received,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(bytes_received)
    }
    
    /// Get socket statistics.
    ///
    /// # Returns
    ///
    /// A Result containing either a tuple with (rx_packets, tx_packets, rx_bytes, tx_bytes) or an error code
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), TcpResult> {
        let mut rx_packets: c_ulonglong = 0;
        let mut tx_packets: c_ulonglong = 0;
        let mut rx_bytes: c_ulonglong = 0;
        let mut tx_bytes: c_ulonglong = 0;
        
        let result = unsafe {
            tcp_socket_get_stats(
                &mut *self.socket as *mut _,
                &mut rx_packets,
                &mut tx_packets,
                &mut rx_bytes,
                &mut tx_bytes,
            )
        };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok((rx_packets, tx_packets, rx_bytes, tx_bytes))
    }
}

impl Drop for TcpSocketWrapper {
    /// Automatically close the socket when it goes out of scope.
    fn drop(&mut self) {
        unsafe {
            tcp_socket_close(&mut *self.socket);
        }
    }
}

/// High-level Rust-friendly TCP socket implementation.
///
/// This structure provides a more idiomatic Rust API around the TCP socket
/// implementation, with error handling and convenient methods.
pub struct VmaTcpSocket {
    inner: TcpSocketWrapper,
}

impl VmaTcpSocket {
    /// Create a new TCP socket with default VMA options.
    ///
    /// # Returns
    ///
    /// A Result containing either the new socket or an error message
    pub fn new() -> Result<Self, String> {
        TcpSocketWrapper::new(None)
            .map(|inner| VmaTcpSocket { inner })
            .map_err(|e| format!("Failed to create TCP socket: {:?}", e))
    }
    
    /// Create a new TCP socket with custom VMA options.
    ///
    /// # Parameters
    ///
    /// * `options` - VMA configuration options
    ///
    /// # Returns
    ///
    /// A Result containing either the new socket or an error message
    pub fn with_options(options: VmaOptions) -> Result<Self, String> {
        TcpSocketWrapper::new(Some(options))
            .map(|inner| VmaTcpSocket { inner })
            .map_err(|e| format!("Failed to create TCP socket with options: {:?}", e))
    }
    
    /// Bind the socket to a local address and port.
    ///
    /// # Parameters
    ///
    /// * `addr` - IP address to bind to
    /// * `port` - Port number to bind to
    ///
    /// # Returns
    ///
    /// A Result containing either success or an error message
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), String> {
        self.inner
            .bind(addr, port)
            .map_err(|e| format!("Failed to bind: {:?}", e))
    }
    
    /// Put the socket in listening mode (server).
    ///
    /// # Parameters
    ///
    /// * `backlog` - Maximum number of pending connections to queue
    ///
    /// # Returns
    ///
    /// A Result containing either success or an error message
    pub fn listen(&mut self, backlog: i32) -> Result<(), String> {
        self.inner
            .listen(backlog)
            .map_err(|e| format!("Failed to listen: {:?}", e))
    }
    
    /// Accept a client connection (server).
    ///
    /// # Parameters
    ///
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either an Option<Client> (None for timeout) or an error message
    pub fn accept(&mut self, timeout: Option<Duration>) -> Result<Option<Client>, String> {
        match self.inner.accept(timeout) {
            Ok(client) => Ok(Some(client)),
            Err(TcpResult::TcpErrorTimeout) => Ok(None), // timeout is not an error
            Err(e) => Err(format!("Failed to accept: {:?}", e)),
        }
    }
    
    /// Connect to a server (client).
    ///
    /// # Parameters
    ///
    /// * `addr` - Server IP address
    /// * `port` - Server port number
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either a boolean (true for connected, false for timeout) or an error message
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16, timeout: Option<Duration>) -> Result<bool, String> {
        match self.inner.connect(addr, port, timeout) {
            Ok(_) => Ok(true),
            Err(TcpResult::TcpErrorTimeout) => Ok(false), // timeout is not an error
            Err(e) => Err(format!("Failed to connect: {:?}", e)),
        }
    }
    
    /// Attempt to reconnect after a disconnection.
    ///
    /// # Parameters
    ///
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either a boolean (true for reconnected, false for timeout/failure) or an error message
    pub fn try_reconnect(&mut self, timeout: Option<Duration>) -> Result<bool, String> {
        match self.inner.reconnect(timeout) {
            Ok(_) => Ok(true),
            Err(TcpResult::TcpErrorTimeout) => Ok(false), // timeout is not an error
            Err(TcpResult::TcpErrorReconnect) => Ok(false), // reconnect failure is treated as a false result
            Err(e) => Err(format!("Failed to reconnect: {:?}", e)),
        }
    }
    
    /// Check if the socket is currently connected.
    ///
    /// # Returns
    ///
    /// True if connected, false otherwise
    pub fn is_connected(&mut self) -> bool {
        self.inner.is_connected()
    }
    
    /// Send data over the connected socket.
    ///
    /// # Parameters
    ///
    /// * `data` - The data to send
    ///
    /// # Returns
    ///
    /// A Result containing either the number of bytes sent or an error message.
    /// Returns 0 bytes if the operation would block.
    pub fn send(&mut self, data: &[u8]) -> Result<usize, String> {
        match self.inner.send(data) {
            Ok(bytes) => Ok(bytes),
            Err(TcpResult::TcpErrorWouldBlock) => Ok(0), // would block is not an error
            Err(e) => Err(format!("Failed to send: {:?}", e)),
        }
    }
    
    /// Receive data from the connected socket.
    ///
    /// # Parameters
    ///
    /// * `buffer` - Buffer to store the received data
    /// * `timeout` - Optional timeout duration
    ///
    /// # Returns
    ///
    /// A Result containing either the number of bytes received or an error message.
    /// Returns 0 bytes for timeout or closed connection.
    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, String> {
        match self.inner.recv(buffer, timeout) {
            Ok(bytes) => Ok(bytes),
            Err(TcpResult::TcpErrorTimeout) => Ok(0), // timeout is not an error
            Err(TcpResult::TcpErrorClosed) => Ok(0), // treat closed as EOF (0 bytes received)
            Err(e) => Err(format!("Failed to receive: {:?}", e)),
        }
    }
    
    /// Get socket statistics.
    ///
    /// # Returns
    ///
    /// A Result containing either a tuple with (rx_packets, tx_packets, rx_bytes, tx_bytes) or an error message
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), String> {
        self.inner
            .get_stats()
            .map_err(|e| format!("Failed to get stats: {:?}", e))
    }
}