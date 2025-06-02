use std::env;
use std::process;
use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use libc::option;
use vma_socket::udp::VmaUdpSocket;
use vma_socket::tcp::VmaTcpSocket;
use vma_socket::common::VmaOptions;

const ITERATIONS: usize = 10000;
const BUFFER_SIZE: usize = 4096;

use flashlog::get_unix_nano;

fn main() {
    let args: Vec<String> = env::args().collect();
    env::set_var("VMA_INTERRUPT_AFFINITY", "0");
    env::set_var("VMA_THREAD_AFFINITY", "0");
    env::set_var("VMA_RX_POLL_OS_RATIO", "1000000");
    
    if args.len() < 2 {
        println!("Usage: {} [udp|tcp] [recv|send|both]", args[0]);
        process::exit(1);
    }

    let protocol = &args[1];
    let operation = args.get(2).map(|s| s.as_str()).unwrap_or("both");

    match protocol.as_str() {
        "udp" => {
            match operation {
                "recv" => benchmark_udp_recv(),
                "send" => benchmark_udp_send(),
                "both" => {
                    benchmark_udp_recv();
                    benchmark_udp_send();
                }
                _ => {
                    println!("Unknown operation: {}", operation);
                    println!("Use 'recv', 'send', or 'both'");
                    process::exit(1);
                }
            }
        },
        "tcp" => {
            match operation {
                "recv" => benchmark_tcp_recv(),
                "send" => benchmark_tcp_send(),
                "both" => {
                    benchmark_tcp_recv();
                    benchmark_tcp_send();
                }
                _ => {
                    println!("Unknown operation: {}", operation);
                    println!("Use 'recv', 'send', or 'both'");
                    process::exit(1);
                }
            }
        }
        _ => {
            println!("Unknown protocol: {}", protocol);
            println!("Use 'udp' or 'tcp'");
            process::exit(1);
        }
    }
}

fn benchmark_udp_recv() {
    println!("=== UDP Recv Polling Benchmark ===");
    
    // Test different configurations
    benchmark_udp_recv_with_options("Low Latency (Polling)", VmaOptions::low_latency());
    benchmark_udp_recv_with_options("High Throughput", VmaOptions::high_throughput());
    
    // Non-polling configuration
    let mut non_polling_options = VmaOptions::default();
    non_polling_options.use_polling = false;
    benchmark_udp_recv_with_options("Non-Polling", non_polling_options);
}


fn benchmark_udp_recv_with_options(config_name: &str, mut options: VmaOptions) {    
    println!("\n--- {} Configuration ---", config_name);
    options.clear_cores();
    options.add_core(0).expect("Failed to set CPU core");
    let mut socket = match VmaUdpSocket::with_options(options.clone()) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create UDP socket: {}", e);
            return;
        }
    };

    // Bind to port
    if let Err(e) = socket.bind("127.0.0.1", 0) { // 0 = auto port allocation
        println!("Failed to bind UDP socket: {}", e);
        return;
    }

    let config_name_clone = config_name.to_string();

    let task = thread::spawn(move || {
        // Set CPU core
        core_affinity::set_for_current(core_affinity::CoreId { id: 0});

        let mut buffer = vec![0u8; BUFFER_SIZE];

        println!("Warming up...");
        // Warmup
        for _ in 0..1000 {
            let _ = if options.use_polling {
                socket.recv(&mut buffer, Some(0)) // 0 = non-blocking for polling
            } else {
                socket.recv(&mut buffer, Some(0)) // 0 = non-blocking for non-polling
            };
        }

        println!("Starting UDP recv benchmark ({} iterations)...", ITERATIONS);
        
        // Single timestamp at the beginning
        let start_time = get_unix_nano();
        
        // recv benchmark (empty socket polling)
        for _i in 0..ITERATIONS {
            socket.recv(&mut buffer, Some(0)).expect("Failed to receive data");
        }

        // Single timestamp at the end
        let end_time = get_unix_nano();
        
        let total_time_ns = end_time - start_time;
        let avg_time_ns = total_time_ns / ITERATIONS as u64;
        let avg_time_us = avg_time_ns as f64 / 1000.0;
        
        println!("UDP Recv ({}) Results:", config_name_clone);
        println!("  Average per operation: {} ns ({:.2} μs)", avg_time_ns, avg_time_us);
    });

    // Wait for the thread to finish
    task.join().expect("Thread panicked");
        
}

fn benchmark_udp_send() {
    println!("\n=== UDP Send Polling Benchmark ===");
    
    // Test different configurations
    benchmark_udp_send_with_options("Low Latency (Polling)", VmaOptions::low_latency());
    benchmark_udp_send_with_options("High Throughput", VmaOptions::high_throughput());
    
    // Non-polling configuration
    let mut non_polling_options = VmaOptions::default();
    non_polling_options.use_polling = false;
    benchmark_udp_send_with_options("Non-Polling", non_polling_options);
}

fn benchmark_udp_send_with_options(config_name: &str, mut options: VmaOptions) {
    println!("\n--- {} Configuration ---", config_name);
    core_affinity::set_for_current(core_affinity::CoreId { id: 0 });
    // Set CPU core
    options.add_core(0).expect("Failed to set CPU core");
    
    // Create socket
    let mut socket = match VmaUdpSocket::with_options(options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create UDP socket: {}", e);
            return;
        }
    };

    socket.connect("127.0.0.1", 12345)
        .expect("Failed to connect UDP socket to dummy port");

    let test_data = b"test";

    println!("Warming up...");
    // Warmup
    for _ in 0..1000 {
        let _ = socket.send(test_data);
    }

    println!("Starting UDP send benchmark ({} iterations)...", ITERATIONS);
    
    // Single timestamp at the beginning
    let start_time = get_unix_nano();
    
    // send benchmark
    for _i in 0..ITERATIONS {
        let _ = socket.send(test_data);
    }

    // Single timestamp at the end
    let end_time = get_unix_nano();
    
    let total_time_ns = end_time - start_time;
    let avg_time_ns = total_time_ns / ITERATIONS as u64;
    let avg_time_us = avg_time_ns as f64 / 1000.0;
    
    println!("UDP Send ({}) Results:", config_name);
    println!("  Average per operation: {} ns ({:.2} μs)", avg_time_ns, avg_time_us);
}

fn benchmark_tcp_recv() {
    println!("=== TCP Recv Benchmark ===");
    
    // Test different configurations
    benchmark_tcp_recv_with_options("Low Latency (Polling)", VmaOptions::low_latency());
    benchmark_tcp_recv_with_options("High Throughput", VmaOptions::high_throughput());
    
    // Non-polling configuration
    let mut non_polling_options = VmaOptions::default();
    non_polling_options.use_polling = false;
    benchmark_tcp_recv_with_options("Non-Polling", non_polling_options);
}

fn benchmark_tcp_recv_with_options(config_name: &str, mut options: VmaOptions) {
    println!("\n--- {} Configuration ---", config_name);
    
    // Set CPU core
    options.add_core(0).expect("Failed to set CPU core");
    
    // Create TCP socket (not connected, just for polling benchmark)
    let mut socket = match VmaTcpSocket::with_options(options.clone()) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create TCP socket: {}", e);
            return;
        }
    };

    let mut buffer = vec![0u8; BUFFER_SIZE];

    println!("Warming up...");
    // Warmup
    for _ in 0..1000 {
        let _ = if options.use_polling {
            socket.recv(&mut buffer, Some(0)) // 0 = non-blocking for polling
        } else {
            socket.recv(&mut buffer, Some(1_000_000)) // 1ms timeout for non-polling
        };
    }

    println!("Starting TCP socket polling benchmark ({} iterations)...", ITERATIONS);
    
    // Single timestamp at the beginning
    let start_time = get_unix_nano();
    
    // recv benchmark (empty socket polling)
    for _i in 0..ITERATIONS {
        let _ = if options.use_polling {
            socket.recv(&mut buffer, Some(0)) // 0 = non-blocking for polling
        } else {
            socket.recv(&mut buffer, Some(0)) // 1ms timeout for non-polling
        };
    }

    // Single timestamp at the end
    let end_time = get_unix_nano();
    
    let total_time_ns = end_time - start_time;
    let avg_time_ns = total_time_ns / ITERATIONS as u64;
    let avg_time_us = avg_time_ns as f64 / 1000.0;
    
    println!("TCP Socket Polling ({}) Results:", config_name);
    println!("  Average per operation: {} ns ({:.2} μs)", avg_time_ns, avg_time_us);
}

fn benchmark_tcp_send() {
    println!("\n=== TCP Send Benchmark ===");
    
    // Test different configurations
    benchmark_tcp_send_with_options("Low Latency (Polling)", VmaOptions::low_latency());
    benchmark_tcp_send_with_options("High Throughput", VmaOptions::high_throughput());
    
    // Non-polling configuration
    let mut non_polling_options = VmaOptions::default();
    non_polling_options.use_polling = false;
    benchmark_tcp_send_with_options("Non-Polling", non_polling_options);
}

fn benchmark_tcp_send_with_options(config_name: &str, mut options: VmaOptions) {
    println!("\n--- {} Configuration ---", config_name);
    
    // Set CPU core
    options.add_core(0).expect("Failed to set CPU core");
    
    // Create TCP socket
    let mut socket = match VmaTcpSocket::with_options(options.clone()) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create TCP socket: {}", e);
            return;
        }
    };

    
    //if let Err(e) = socket.connect("127.0.0.1", 12345, None) {
    //    println!("Failed to connect TCP socket: {}", e);
    //    return;
    //}
    let test_data = b"test";
    println!("Warming up...");
    // Warmup
    for _ in 0..1000 {
        let _ = socket.send(test_data);
    }
    println!("Starting TCP send benchmark ({} iterations)...", ITERATIONS);
    // Single timestamp at the beginning

    let start_time = get_unix_nano();
    // send benchmark
    for _i in 0..ITERATIONS {
        let _ = socket.send(test_data);
    }
    // Single timestamp at the end
    let end_time = get_unix_nano();
    let total_time_ns = end_time - start_time;
    let avg_time_ns = total_time_ns / ITERATIONS as u64;
    let avg_time_us = avg_time_ns as f64 / 1000.0;
    println!("TCP Send ({}) Results:", config_name);
    println!("  Average per operation: {} ns ({:.2} μs)", avg_time_ns, avg_time_us);
}