# vma_socket

A Rust wrapper for the Mellanox VMA (Messaging Accelerator) library, providing high-performance networking capabilities for ultra-low latency applications.

## Overview

`vma_socket` provides a safe and ergonomic Rust interface to the VMA library, which leverages RDMA (Remote Direct Memory Access) technology to bypass kernel overhead and deliver extremely low latency networking. This library is particularly useful for:

- High-frequency trading applications
- Real-time financial systems
- Low-latency messaging platforms
- High-performance computing
- Network appliances

## Features

- **High-level Rust API**: Ergonomic Rust wrappers around the VMA C library
- **Ultra-low latency**: Direct access to VMA's performance optimizations
- **Comprehensive socket types**: Support for both UDP and TCP sockets
- **Safe abstractions**: Memory-safe wrappers around the C implementation
- **Configurable options**: Fine-grained control of VMA performance parameters
- **Detailed statistics**: Access to packet and byte counters for monitoring

## Requirements

- Mellanox RDMA-capable network adapter (ConnectX-3/4/5/6/7 or better)
- Mellanox OFED drivers installed
- VMA library (`libvma.so`) installed
- Linux environment (VMA is Linux-only)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vma_socket = "0.1.0"
```

Make sure you have the VMA library installed on your system. The VMA library is typically installed with Mellanox OFED packages.

## Usage Examples

### UDP Example

```rust
use std::time::Duration;
use vma_socket::udp::VmaUdpSocket;
use vma_socket::common::VmaOptions;

// Create socket with low-latency optimizations
let vma_options = VmaOptions::low_latency();
let mut socket = VmaUdpSocket::with_options(vma_options)?;

// For a server application
socket.bind("0.0.0.0", 5001)?;

// For a client application
socket.connect("192.168.1.100", 5001)?;
socket.send("Hello VMA!".as_bytes())?;

// Receive data with timeout
let mut buffer = vec![0; 4096];
match socket.recv_from(&mut buffer, Some(Duration::from_millis(100)))? {
    Some(packet) => println!("Received {} bytes from {}", packet.data.len(), packet.src_addr),
    None => println!("No packet received (timeout)"),
}
```

### TCP Example

```rust
use std::time::Duration;
use vma_socket::tcp::VmaTcpSocket;
use vma_socket::common::VmaOptions;

// Server example
let mut server = VmaTcpSocket::with_options(VmaOptions::low_latency())?;
server.bind("0.0.0.0", 5002)?;
server.listen(10)?;

if let Ok(Some(mut client)) = server.accept(Some(Duration::from_secs(1))) {
    println!("Connection from {}", client.address);
    
    let mut buffer = vec![0u8; 1024];
    let received = client.recv(&mut buffer, Some(Duration::from_millis(100)))?;
    
    if received > 0 {
        client.send(&buffer[0..received])?;
    }
}

// Client example
let mut client = VmaTcpSocket::with_options(VmaOptions::low_latency())?;
if client.connect("192.168.1.100", 5002, Some(Duration::from_secs(5)))? {
    client.send("Hello VMA TCP!".as_bytes())?;
}
```

## Performance Tuning

For optimal performance, you may want to:

1. Use the `VmaOptions::low_latency()` or `VmaOptions::high_throughput()` factory methods
2. Enable `use_socketxtreme` for maximum performance (requires proper VMA configuration)
3. Adjust buffer sizes to match your application's needs
4. Set appropriate CPU affinity for your networking threads
5. Experiment with polling vs interrupt-driven modes

Example of custom options configuration:

```rust
let options = VmaOptions {
    use_socketxtreme: true,
    optimize_for_latency: true,
    use_polling: true,
    ring_count: 1, // Single ring for lower latency
    buffer_size: 8192, // Smaller buffers
    enable_timestamps: true,
    use_hugepages: true,
    tx_bufs: 32,
    rx_bufs: 16,
    disable_poll_yield: true,
    skip_os_select: true,
    keep_qp_full: true,
    cpu_cores: std::ptr::null_mut(),
    cpu_cores_count: 0,
};
```

## Running with VMA

There are two ways to run your application with VMA:

### Using LD_PRELOAD manually

To run your Rust application with VMA, use the `LD_PRELOAD` environment variable:

```bash
LD_PRELOAD=/usr/lib64/libvma.so.x.x.x ./your_application
```

### Using the provided run.sh script

The library includes a convenience script `run.sh` that builds and runs the examples with VMA enabled:

To run the UDP server and client examples, open two terminals:

Terminal 1 (server):
```bash
# Start the UDP server
./run.sh udp_test server
```

Terminal 2 (client):
```bash
# Run the UDP client connecting to the server
./run.sh udp_test client
```

Similarly for TCP examples:

Terminal 1 (server):
```bash
# Start the TCP server
./run.sh tcp_test server
```

Terminal 2 (client):
```bash
# Run the TCP client connecting to the server
./run.sh tcp_test client
```

You can also specify IP addresses and ports:
```bash
./run.sh tcp_test server 0.0.0.0 5002  # Server listening on all interfaces
./run.sh tcp_test client 192.168.1.100 5002  # Client connecting to a specific IP
```

The script will:
1. Build the specified example using cargo
2. Run it with the VMA library pre-loaded
3. Pass all additional arguments to the example program

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
