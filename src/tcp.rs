use std::ffi::{c_void, CString};

// The Client structure represents a connected client (for server sockets)
pub struct Client {
    inner: Box<TcpClient>,
    pub address: SocketAddr,
}

impl Client {
    fn new(client: TcpClient) -> Self {
        let address = sockaddr_to_rust(&client.addr);
        Client {
            inner: Box::new(client),
            address,
        }
    }
    
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
    
    pub fn close(&mut self) -> Result<(), TcpResult> {
        let result = unsafe { tcp_socket_close_client(&mut *self.inner) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if self.inner.socket_fd >= 0 {
            unsafe {
                tcp_socket_close_client(&mut *self.inner);
            }
        }
    }
}

// Low-level wrapper over the C socket
pub struct TcpSocketWrapper {
    socket: Box<TcpSocket>,
}

impl TcpSocketWrapper {
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
    
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), TcpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { tcp_socket_bind(&mut *self.socket, c_addr.as_ptr(), port) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    pub fn listen(&mut self, backlog: i32) -> Result<(), TcpResult> {
        let result = unsafe { tcp_socket_listen(&mut *self.socket, backlog) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    pub fn accept(&mut self, timeout: Option<Duration>) -> Result<Client, TcpResult> {
        let mut client = unsafe { mem::zeroed::<TcpClient>() };
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe { tcp_socket_accept(&mut *self.socket, &mut client, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(Client::new(client))
    }
    
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16, timeout: Option<Duration>) -> Result<(), TcpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let timeout_ms = duration_to_ms(timeout);
        
        let result = unsafe { tcp_socket_connect(&mut *self.socket, c_addr.as_ptr(), port, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    pub fn reconnect(&mut self, timeout: Option<Duration>) -> Result<(), TcpResult> {
        let timeout_ms = duration_to_ms(timeout);
        let result = unsafe { tcp_socket_reconnect(&mut *self.socket, timeout_ms) };
        
        if result != TcpResult::TcpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }
    
    pub fn is_connected(&mut self) -> bool {
        unsafe { tcp_socket_is_connected(&mut *self.socket) }
    }
    
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
    fn drop(&mut self) {
        unsafe {
            tcp_socket_close(&mut *self.socket);
        }
    }
}

// High-level safe wrapper API
pub struct VmaTcpSocket {
    inner: TcpSocketWrapper,
}

impl VmaTcpSocket {
    pub fn new() -> Result<Self, String> {
        TcpSocketWrapper::new(None)
            .map(|inner| VmaTcpSocket { inner })
            .map_err(|e| format!("Failed to create TCP socket: {:?}", e))
    }
    
    pub fn with_options(options: VmaOptions) -> Result<Self, String> {
        TcpSocketWrapper::new(Some(options))
            .map(|inner| VmaTcpSocket { inner })
            .map_err(|e| format!("Failed to create TCP socket with options: {:?}", e))
    }
    
    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), String> {
        self.inner
            .bind(addr, port)
            .map_err(|e| format!("Failed to bind: {:?}", e))
    }
    
    pub fn listen(&mut self, backlog: i32) -> Result<(), String> {
        self.inner
            .listen(backlog)
            .map_err(|e| format!("Failed to listen: {:?}", e))
    }
    
    pub fn accept(&mut self, timeout: Option<Duration>) -> Result<Option<Client>, String> {
        match self.inner.accept(timeout) {
            Ok(client) => Ok(Some(client)),
            Err(TcpResult::TcpErrorTimeout) => Ok(None), // timeout is not an error
            Err(e) => Err(format!("Failed to accept: {:?}", e)),
        }
    }
    
    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16, timeout: Option<Duration>) -> Result<bool, String> {
        match self.inner.connect(addr, port, timeout) {
            Ok(_) => Ok(true),
            Err(TcpResult::TcpErrorTimeout) => Ok(false), // timeout is not an error
            Err(e) => Err(format!("Failed to connect: {:?}", e)),
        }
    }
    
    pub fn try_reconnect(&mut self, timeout: Option<Duration>) -> Result<bool, String> {
        match self.inner.reconnect(timeout) {
            Ok(_) => Ok(true),
            Err(TcpResult::TcpErrorTimeout) => Ok(false), // timeout is not an error
            Err(TcpResult::TcpErrorReconnect) => Ok(false), // reconnect failure is treated as a false result
            Err(e) => Err(format!("Failed to reconnect: {:?}", e)),
        }
    }
    
    pub fn is_connected(&mut self) -> bool {
        self.inner.is_connected()
    }
    
    pub fn send(&mut self, data: &[u8]) -> Result<usize, String> {
        match self.inner.send(data) {
            Ok(bytes) => Ok(bytes),
            Err(TcpResult::TcpErrorWouldBlock) => Ok(0), // would block is not an error
            Err(e) => Err(format!("Failed to send: {:?}", e)),
        }
    }
    
    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, String> {
        match self.inner.recv(buffer, timeout) {
            Ok(bytes) => Ok(bytes),
            Err(TcpResult::TcpErrorTimeout) => Ok(0), // timeout is not an error
            Err(TcpResult::TcpErrorClosed) => Ok(0), // treat closed as EOF (0 bytes received)
            Err(e) => Err(format!("Failed to receive: {:?}", e)),
        }
    }
    
    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), String> {
        self.inner
            .get_stats()
            .map_err(|e| format!("Failed to get stats: {:?}", e))
    }
}

use std::mem;
use std::net::SocketAddr;
use std::os::raw::{c_char, c_int, c_ulonglong};
use std::time::Duration;

use crate::common::{SockAddrIn, VmaOptions, duration_to_ms, sockaddr_to_rust};

// Connection state enum
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TcpConnectionState {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Listening = 3,
}

// TCP VMA options
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

// TCP socket structure
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

// Client connection structure
#[repr(C)]
pub struct TcpClient {
    pub socket_fd: c_int,
    pub addr: SockAddrIn,
    pub rx_bytes: c_ulonglong,
    pub tx_bytes: c_ulonglong,
}

// Result codes
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