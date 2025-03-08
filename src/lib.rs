// src/lib.rs
use std::ffi::{c_void, CStr, CString};
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong};
use std::ptr;
use std::time::Duration;

// C 라이브러리의 타입 정의
#[repr(C)]
pub struct UdpVmaOptions {
    pub use_socketxtreme: bool,
    pub optimize_for_latency: bool,
    pub use_polling: bool,
    pub ring_count: c_int,
    pub buffer_size: c_int,
    pub enable_timestamps: bool,
}

#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

#[repr(C)]
pub struct UdpSocket {
    pub socket_fd: c_int,
    pub vma_options: UdpVmaOptions,
    pub local_addr: SockAddrIn,
    pub remote_addr: SockAddrIn,
    pub is_bound: bool,
    pub is_connected: bool,
    pub rx_packets: c_ulonglong,
    pub tx_packets: c_ulonglong,
    pub rx_bytes: c_ulonglong,
    pub tx_bytes: c_ulonglong,
}

#[repr(C)]
pub struct UdpPacket {
    pub data: *mut c_void,
    pub length: usize,
    pub src_addr: SockAddrIn,
    pub timestamp: c_ulonglong,
}

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

// C 함수 외부 선언
extern "C" {
    fn udp_socket_init(socket: *mut UdpSocket, options: *const UdpVmaOptions) -> c_int;
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

// Rust 래퍼 구현
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
            buffer_size: 65536,
            enable_timestamps: true,
        }
    }
}

impl From<VmaOptions> for UdpVmaOptions {
    fn from(opts: VmaOptions) -> Self {
        UdpVmaOptions {
            use_socketxtreme: opts.use_socketxtreme,
            optimize_for_latency: opts.optimize_for_latency,
            use_polling: opts.use_polling,
            ring_count: opts.ring_count,
            buffer_size: opts.buffer_size,
            enable_timestamps: opts.enable_timestamps,
        }
    }
}

pub struct Packet {
    pub data: Vec<u8>,
    pub src_addr: SocketAddr,
    pub timestamp: u64,
}

pub struct UdpSocketWrapper {
    socket: Box<UdpSocket>,
}

impl UdpSocketWrapper {
    pub fn new(options: Option<VmaOptions>) -> Result<Self, UdpResult> {
        let mut socket = Box::new(unsafe { mem::zeroed::<UdpSocket>() });
        
        let c_options: UdpVmaOptions = match options {
            Some(opts) => opts.into(),
            None => VmaOptions::default().into(),
        };

        let result = unsafe { udp_socket_init(&mut *socket, &c_options) };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(UdpSocketWrapper { socket })
    }

    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), UdpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { udp_socket_bind(&mut *self.socket, c_addr.as_ptr(), port) };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }

    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), UdpResult> {
        let c_addr = CString::new(addr.into()).unwrap();
        let result = unsafe { udp_socket_connect(&mut *self.socket, c_addr.as_ptr(), port) };
        
        if result != UdpResult::UdpSuccess as i32 {
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(())
    }

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
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(bytes_sent)
    }

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
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(bytes_sent)
    }

    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, UdpResult> {
        let mut bytes_received: usize = 0;
        let timeout_ms = match timeout {
            Some(t) => t.as_millis() as c_int,
            None => -1, // 무한 대기
        };
        
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
            return Err(unsafe { mem::transmute(result) });
        }
        
        Ok(bytes_received)
    }

    pub fn recv_from(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<Packet, UdpResult> {
        let mut packet = unsafe { mem::zeroed::<UdpPacket>() };
        let timeout_ms = match timeout {
            Some(t) => t.as_millis() as c_int,
            None => -1, // 무한 대기
        };
        
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
            return Err(unsafe { mem::transmute(result) });
        }
        
        // 주소 변환
        let ip = Ipv4Addr::from(u32::from_be(packet.src_addr.sin_addr));
        let port = u16::from_be(packet.src_addr.sin_port);
        let addr = SocketAddr::new(IpAddr::V4(ip), port);
        
        // 데이터 복사
        let data = unsafe { std::slice::from_raw_parts(packet.data as *const u8, packet.length) }.to_vec();
        
        Ok(Packet {
            data,
            src_addr: addr,
            timestamp: packet.timestamp,
        })
    }

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
            return Err(unsafe { mem::transmute(result) });
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

// 안전한 래퍼 - 상위 레벨 Rust API
pub struct VmaUdpSocket {
    inner: UdpSocketWrapper,
}

impl VmaUdpSocket {
    pub fn new() -> Result<Self, String> {
        UdpSocketWrapper::new(None)
            .map(|inner| VmaUdpSocket { inner })
            .map_err(|e| format!("Failed to create socket: {:?}", e))
    }

    pub fn with_options(options: VmaOptions) -> Result<Self, String> {
        UdpSocketWrapper::new(Some(options))
            .map(|inner| VmaUdpSocket { inner })
            .map_err(|e| format!("Failed to create socket with options: {:?}", e))
    }

    pub fn bind<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), String> {
        self.inner
            .bind(addr, port)
            .map_err(|e| format!("Failed to bind: {:?}", e))
    }

    pub fn connect<A: Into<String>>(&mut self, addr: A, port: u16) -> Result<(), String> {
        self.inner
            .connect(addr, port)
            .map_err(|e| format!("Failed to connect: {:?}", e))
    }

    pub fn send(&mut self, data: &[u8]) -> Result<usize, String> {
        self.inner
            .send(data)
            .map_err(|e| format!("Failed to send: {:?}", e))
    }

    pub fn send_to<A: Into<String>>(&mut self, data: &[u8], addr: A, port: u16) -> Result<usize, String> {
        self.inner
            .send_to(data, addr, port)
            .map_err(|e| format!("Failed to send to address: {:?}", e))
    }

    pub fn recv(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<usize, String> {
        match self.inner.recv(buffer, timeout) {
            Ok(bytes) => Ok(bytes),
            Err(UdpResult::UdpErrorTimeout) => Ok(0), // 타임아웃은 오류가 아님
            Err(e) => Err(format!("Failed to receive: {:?}", e)),
        }
    }

    pub fn recv_from(&mut self, buffer: &mut [u8], timeout: Option<Duration>) -> Result<Option<Packet>, String> {
        match self.inner.recv_from(buffer, timeout) {
            Ok(packet) => Ok(Some(packet)),
            Err(UdpResult::UdpErrorTimeout) => Ok(None), // 타임아웃은 오류가 아님
            Err(e) => Err(format!("Failed to receive from: {:?}", e)),
        }
    }

    pub fn get_stats(&mut self) -> Result<(u64, u64, u64, u64), String> {
        self.inner
            .get_stats()
            .map_err(|e| format!("Failed to get stats: {:?}", e))
    }
}