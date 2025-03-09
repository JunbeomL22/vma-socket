//! # VMA Socket
//!
//! A Rust wrapper library for high-performance networking using the Mellanox VMA (Messaging Accelerator) library.
//!
//! This library provides both UDP and TCP socket implementations that leverage VMA for extremely
//! low-latency and high-throughput networking on compatible hardware. The implementation includes
//! both low-level FFI bindings to the C VMA library and high-level, safe Rust abstractions.
//!
//! ## Features
//!
//! - High-performance UDP sockets optimized for low-latency applications
//! - High-performance TCP sockets with connection management
//! - Easy-to-use Rust API with safe abstractions
//! - VMA acceleration leveraging RDMA capabilities
//! - Fine-grained control over socket parameters
//!
//! ## Requirements
//!
//! - Mellanox OFED drivers and libraries
//! - VMA library (libvma)
//! - Compatible Mellanox network adapters
//!
//! ## Example: UDP Server
//!
//! ```rust
//! use std::time::Duration;
//! use vma_socket::udp::{VmaOptions, VmaUdpSocket};
//!
//! fn run_udp_server() {
//!     // Create socket with custom VMA options for maximum performance
//!     let vma_options = VmaOptions {
//!         use_socketxtreme: true,
//!         optimize_for_latency: true,
//!         use_polling: true,
//!         ring_count: 4,
//!         buffer_size: 4096,
//!         enable_timestamps: true,
//!     };
//!
//!     // Create and bind the socket
//!     let mut socket = VmaUdpSocket::with_options(vma_options).unwrap();
//!     socket.bind("0.0.0.0", 5001).unwrap();
//!     println!("UDP server listening on 0.0.0.0:5001");
//!
//!     // Receive buffer
//!     let mut buffer = vec![0u8; 4096];
//!     
//!     loop {
//!         // Wait for incoming packets with 100ms timeout
//!         match socket.recv_from(&mut buffer, Some(Duration::from_millis(100))) {
//!             Ok(Some(packet)) => {
//!                 println!("Received {} bytes from {}", packet.data.len(), packet.src_addr);
//!                 
//!                 // Echo data back to sender
//!                 let ip = packet.src_addr.ip().to_string();
//!                 let port = packet.src_addr.port();
//!                 socket.send_to(&packet.data, ip, port).unwrap();
//!             },
//!             Ok(None) => {
//!                 // Timeout - continue
//!             },
//!             Err(e) => {
//!                 eprintln!("Error: {}", e);
//!                 break;
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Example: TCP Server
//!
//! ```rust
//! use std::time::Duration;
//! use vma_socket::tcp::VmaTcpSocket;
//! use vma_socket::common::VmaOptions;
//!
//! fn run_tcp_server() {
//!     // Create socket with default VMA options
//!     let mut socket = VmaTcpSocket::new().unwrap();
//!     
//!     // Bind and listen
//!     socket.bind("0.0.0.0", 5002).unwrap();
//!     socket.listen(10).unwrap();
//!     println!("TCP server listening on 0.0.0.0:5002");
//!     
//!     // Accept and handle clients
//!     while let Ok(Some(mut client)) = socket.accept(Some(Duration::from_secs(1))) {
//!         println!("Connection from {}", client.address);
//!         
//!         // Receive buffer
//!         let mut buffer = vec![0u8; 1024];
//!         
//!         // Echo server - read data and send it back
//!         match client.recv(&mut buffer, Some(Duration::from_millis(100))) {
//!             Ok(len) if len > 0 => {
//!                 println!("Received {} bytes", len);
//!                 client.send(&buffer[0..len]).unwrap();
//!             },
//!             Ok(_) => println!("No data or connection closed"),
//!             Err(e) => println!("Error: {}", e),
//!         }
//!     }
//! }
//! ```
//!
//! ## Module Structure
//!
//! - `udp`: High-performance UDP socket implementation
//! - `tcp`: High-performance TCP socket implementation
//! - `common`: Shared types and utilities used by both implementations

/// UDP socket implementation
pub mod udp;

/// TCP socket implementation
pub mod tcp;

/// Common types and utilities
pub mod common;
