# VMA Socket

A Rust wrapper library providing high-performance network sockets using the RDMA-accelerated Mellanox Messaging Accelerator (VMA) library.

## Overview

VMA Socket provides a safe and ergonomic Rust interface to the VMA library, which offers extremely low-latency and high-throughput networking on supported hardware. This library includes both low-level bindings to the C VMA library as well as high-level Rust abstractions.

## Features

- **High-performance UDP sockets**: Optimized for low-latency and high-throughput applications
- **High-performance TCP sockets**: With connection management and streaming capabilities
- **Easy-to-use Rust API**: Safe abstractions over the C bindings
- **VMA Acceleration**: Takes advantage of RDMA capabilities on compatible hardware
- **Fine-grained control**: Configure VMA options for your specific use case
- **Zero-copy**: Minimizes memory operations for maximum performance

## Prerequisites

- Mellanox OFED drivers and libraries installed
- VMA library (libvma) installed
- Compatible Mellanox network adapters

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vma_socket = "0.1.0"
```

Ensure that the VMA library is installed on your system. This is typically available through the Mellanox OFED package.

## Usage Examples

### UDP Example

```rust
use std::time::Duration;
use vma_socket::udp::{VmaOptions, VmaUdpSocket};

// Create a UDP socket with custom VMA options
let vma_options = VmaOptions {
    use_socketxtreme: true,
    optimize_for_latency: true,
    use_polling: true,
    ring_count: 4,
    buffer_size: 4096,
    enable_timestamps: true,
};

// Create and bind the socket
let mut socket = VmaUdpSocket::with_options(vma_options).unwrap();
socket.bind("127.0.0.1", 5001).unwrap();

// For a server (receiving)
let mut buffer = vec![0u8; 4096];
match socket.recv_from(&mut buffer, Some(Duration::from_millis(100))) {
    Ok(Some(packet)) => {
        println!("Received {} bytes from {}", packet.data.len(), packet.src_addr);
    },
    Ok(None) => println!("Timeout"),
    Err(e) => println!("Error: {}", e),
}

// For a client (sending)
socket.connect("127.0.0.1", 5001).unwrap();
let data = vec![1, 2, 3, 4];
socket.send(&data).unwrap();
```

### TCP Example

```rust
use std::time::Duration;
use vma_socket::tcp::{VmaTcpSocket};
use vma_socket::common::VmaOptions;

// Create a TCP socket with default VMA options
let mut socket = VmaTcpSocket::new().unwrap();

// Server mode
socket.bind("0.0.0.0", 5002).unwrap();
socket.listen(10).unwrap();

match socket.accept(Some(Duration::from_secs(1))) {
    Ok(Some(client)) => {
        println!("Connection from {}", client.address);
        let mut buffer = vec![0u8; 1024];
        match client.recv(&mut buffer, Some(Duration::from_millis(100))) {
            Ok(len) => println!("Received {} bytes", len),
            Err(e) => println!("Error: {}", e),
        }
    },
    Ok(None) => println!("No connections within timeout"),
    Err(e) => println!("Error: {}", e),
}

// Client mode
let mut socket = VmaTcpSocket::new().unwrap();
match socket.connect("127.0.0.1", 5002, Some(Duration::from_secs(5))) {
    Ok(true) => {
        println!("Connected!");
        socket.send(b"Hello VMA").unwrap();
    },
    Ok(false) => println!("Connection timeout"),
    Err(e) => println!("Error: {}", e),
}
```

## Running the Examples

The repository includes example programs for both UDP and TCP in both Rust and C implementations. To run them:

```bash
# Ensure VMA is in your LD_PRELOAD path
export LD_PRELOAD=/usr/lib64/libvma.so.9.8.51

# UDP test in server mode
cargo run --example udp_test -- server 127.0.0.1 5001

# UDP test in client mode
cargo run --example udp_test -- client 127.0.0.1 5001

# TCP test in server mode
cargo run --example tcp_test -- server 127.0.0.1 5002

# TCP test in client mode
cargo run --example tcp_test -- client 127.0.0.1 5002
```

## Performance Tuning

For optimal performance:

1. Use `use_socketxtreme: true` for single-threaded applications
2. Set `optimize_for_latency: true` for latency-sensitive applications
3. Enable polling with `use_polling: true` for reduced latency at the cost of higher CPU usage
4. Adjust `ring_count` based on your expected concurrency
5. Set an appropriate `buffer_size` for your specific application needs

## Building from Source

```bash
git clone https://github.com/yourusername/vma_socket.git
cd vma_socket
cargo build --release
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Mellanox for the VMA library and RDMA technology
- The Rust community for the excellent FFI support