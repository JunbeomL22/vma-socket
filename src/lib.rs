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
//! use std::time::Duration;
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
//!     match socket.recv_from(&mut buffer, Some(Duration::from_millis(100)))? {
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
//! use std::time::Duration;
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
//!     if let Some(mut client) = socket.accept(Some(Duration::from_secs(1)))? {
//!         println!("Connection from {}", client.address);
//!         
//!         // Receive data
//!         let mut buffer = vec![0u8; 1024];
//!         let received = client.recv(&mut buffer, Some(Duration::from_millis(100)))?;
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

/// UDP socket implementation
pub mod udp;

/// TCP socket implementation
pub mod tcp;

/// Common types and utilities
pub mod common;
