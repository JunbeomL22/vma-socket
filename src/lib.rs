//! # vma-socket
//!
//! A flexible high-level Rust API for TCP/UDP sockets leveraging Mellanox/NVIDIA VMA 
//! (Messaging Accelerator) for low-latency networking.
//!
//! This library provides clean, safe Rust wrappers around the VMA C library, allowing
//! developers to use the performance benefits of VMA without dealing with the complexity
//! of direct C FFI calls.
//!
//! ## Features
//!
//! - Simple, idiomatic Rust API for UDP and TCP sockets
//! - Configurable performance options to match application requirements
//! - Timeout handling and error management in Rust style
//! - Memory safety through proper Rust abstractions
//!
//! ## UDP Example
//!
//! ```rust,no_run
//! use vma_socket::udp::VmaUdpSocket;
//! 
//! fn udp_example() -> Result<(), String> {
//!     // Create a socket with default options
//!     let mut socket = VmaUdpSocket::new()?;
//!     
//!     // Bind to a port
//!     socket.bind("0.0.0.0", 5001)?;
//!     
//!     // Receive data with timeout
//!     let mut buffer = vec![0u8; 4096];
//!     match socket.recv_from(&mut buffer, Some(100_000_000)) { // 100ms timeout
//!         Some(packet) => {
//!             println!("Received {} bytes", packet.data.len());
//!             
//!             // Reply to sender
//!             let ip = packet.src_addr.ip().to_string();
//!             let port = packet.src_addr.port();
//!             socket.send_to(&packet.data, ip, port)?;
//!         },
//!         None => println!("No data received (timeout)"),
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
//! 
//! fn tcp_server_example() -> Result<(), String> {
//!     // Create a TCP socket
//!     let mut socket = VmaTcpSocket::new()?;
//!     
//!     // Bind and listen
//!     socket.bind("0.0.0.0", 5002)?;
//!     socket.listen(10)?;
//!     
//!     // Accept client connections
//!     if let Some(mut client) = socket.accept(Some(1_000_000_000))? {
//!         // Receive and echo data
//!         let mut buffer = vec![0u8; 1024];
//!         let received = client.recv(&mut buffer, Some(100_000_000))?; // 100ms timeout
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
//! To utilize VMA acceleration, run your application with the VMA library preloaded:
//!
//! ```bash
//! LD_PRELOAD=/usr/lib64/libvma.so.x.x.x ./your_application
//! ```
//!
//! ## Module Structure
//!
//! - [`udp`]: UDP socket implementation
//! - [`tcp`]: TCP socket implementation
//! - [`common`]: Shared types and configuration options

/// UDP socket implementation
pub mod udp;

/// TCP socket implementation
pub mod tcp;

/// Common types and utilities
pub mod common;