# vma-socket

A flexible high-level Rust API for TCP/UDP sockets leveraging Mellanox/NVIDIA VMA (Messaging Accelerator) for low-latency networking.

## Overview

`vma-socket` provides a clean, ergonomic Rust interface to the VMA library, allowing developers to access the performance benefits of VMA while working with a familiar and safe API. The library offers:

- Simple, idiomatic Rust wrappers for UDP and TCP sockets
- Configurable options for different performance requirements
- Flexible timeout and polling mechanisms
- Safe memory handling and error management

## Requirements

- Mellanox RDMA-capable network adapter
- Mellanox OFED drivers
- VMA library (`libvma.so`)
- Linux environment

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vma-socket = "0.1.0"
```

## Basic Usage

### UDP Example

```rust
use std::time::Duration;
use vma_socket::udp::VmaUdpSocket;
use vma_socket::common::VmaOptions;

// Create a UDP socket
let mut socket = VmaUdpSocket::new()?;

// Or with custom options
let options = VmaOptions::low_latency();
let mut socket = VmaUdpSocket::with_options(options)?;

// Server: bind to an address
socket.bind("0.0.0.0", 5001)?;

// Client: connect to a server
socket.connect("192.168.1.100", 5001)?;
socket.send("Hello".as_bytes())?;

// Receive data with timeout
let mut buffer = vec![0; 4096];
match socket.recv_from(&mut buffer, Some(Duration::from_millis(100)))? {
    Some(packet) => println!("Received: {}", 
                            String::from_utf8_lossy(&packet.data)),
    None => println!("No data received (timeout)"),
}
```

### TCP Example

```rust
use std::time::Duration;
use vma_socket::tcp::VmaTcpSocket;
use vma_socket::common::VmaOptions;

// Server example
let mut server = VmaTcpSocket::new()?;
server.bind("0.0.0.0", 5002)?;
server.listen(10)?;

if let Some(mut client) = server.accept(Some(Duration::from_secs(1)))? {
    let mut buffer = vec![0u8; 1024];
    let received = client.recv(&mut buffer, Some(Duration::from_millis(100)))?;
    client.send(&buffer[0..received])?;
}

// Client example
let mut client = VmaTcpSocket::new()?;
if client.connect("192.168.1.100", 5002, Some(Duration::from_secs(5)))? {
    client.send("Hello".as_bytes())?;
}
```

## Configuration Options

The library allows flexible configuration:

```rust
// Use predefined profiles
let low_latency = VmaOptions::low_latency();
let high_throughput = VmaOptions::high_throughput();

// Or customize your own
let custom_options = VmaOptions {
    use_socketxtreme: true,
    optimize_for_latency: true,
    use_polling: true,
    ring_count: 2,
    buffer_size: 8192,
    // ... other options
};
```

## Running with VMA

To use the VMA acceleration, preload the VMA library when running your application:

```bash
LD_PRELOAD=/usr/lib64/libvma.so.x.x.x ./your_application
```

### Development Helper Script

For testing examples, use the included run.sh script:

```bash
# Terminal 1: Run UDP server
./run.sh udp_test [server|client] [address] [port]

# Terminal 2: Run UDP client
./run.sh udp_test [server|client] [address] [port]
```

You can also specify addresses and ports:
```bash
./run.sh tcp_test server 0.0.0.0 5002
./run.sh tcp_test client 192.168.1.100 5002
```

## License

This project is licensed under the MIT or Apache-2.0 License.