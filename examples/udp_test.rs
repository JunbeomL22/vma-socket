use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vma_socket::udp::VmaUdpSocket;
use vma_socket::common::VmaOptions;

const BUFFER_SIZE: usize = 4096;
const TEST_DURATION: u64 = 10; // Test duration in seconds

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} [server|client] [ip] [port]", args[0]);
        println!("  Default: 127.0.0.1:5001");
        process::exit(1);
    }

    let mode = &args[1];
    let ip = args.get(2).map(|s| s.as_str()).unwrap_or("127.0.0.1");
    let port: u16 = args.get(3).map(|s| s.parse().unwrap_or(5001)).unwrap_or(5001);

    // Handle Ctrl-C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("Received termination signal, ending test...");
    })
    .expect("Error setting Ctrl-C handler");

    match mode.as_str() {
        "server" => run_server(running, ip, port),
        "client" => run_client(running, ip, port),
        _ => {
            println!("Unknown mode: {}", mode);
            println!("Use 'server' or 'client'");
            process::exit(1);
        }
    }
}

fn run_server(running: Arc<AtomicBool>, ip: &str, port: u16) {
    println!("Server mode (receiving): {}:{}", ip, port);

    // Set VMA options - using low latency profile
    let vma_options = VmaOptions::low_latency();

    // Create UDP socket with detailed error handling
    let mut socket = match VmaUdpSocket::with_options(vma_options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create socket: {}", e);
            return;
        }
    };

    // Bind to address
    if let Err(e) = socket.bind(ip, port) {
        println!("Failed to bind: {}", e);
        return;
    }

    println!("UDP server listening on {}:{}", ip, port);

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut packets_received = 0u64;
    let mut bytes_received = 0u64;

    let start_time = std::time::Instant::now();

    // Receiving loop
    while running.load(Ordering::SeqCst) && start_time.elapsed().as_secs() < TEST_DURATION {
        match socket.recv_from(&mut buffer, Some(Duration::from_millis(100))) {
            Ok(Some(packet)) => {
                packets_received += 1;
                bytes_received += packet.data.len() as u64;

                if packets_received % 10000 == 0 {
                    println!("Received {} packets", packets_received);
                }
            }
            Ok(None) => {
                // Timeout - continue
            }
            Err(e) => {
                println!("Receive error: {}", e);
                break;
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!("\n====== Test Results ======");
    println!("Total packets received: {}", packets_received);
    println!("Total bytes received: {}", bytes_received);
    println!("Packets per second: {:.2}", packets_received as f64 / elapsed);
    println!("Throughput: {:.2} Mbps", 8.0 * bytes_received as f64 / elapsed / 1_000_000.0);
}

fn run_client(running: Arc<AtomicBool>, ip: &str, port: u16) {
    println!("Client mode (sending): {}:{}", ip, port);

    // Set VMA options - using low latency profile
    let vma_options = VmaOptions::low_latency();

    // Create UDP socket with detailed error handling
    let mut socket = match VmaUdpSocket::with_options(vma_options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create socket: {}", e);
            return;
        }
    };

    // Connect to target address
    if let Err(e) = socket.connect(ip, port) {
        println!("Failed to connect: {}", e);
        return;
    }

    println!("UDP client sending data to {}:{}", ip, port);

    // Create test data
    let data = vec![b'A'; BUFFER_SIZE];
    let mut packets_sent = 0u64;
    let mut bytes_sent = 0u64;

    let start_time = std::time::Instant::now();

    // Sending loop
    while running.load(Ordering::SeqCst) && start_time.elapsed().as_secs() < TEST_DURATION {
        match socket.send(&data) {
            Ok(sent) => {
                packets_sent += 1;
                bytes_sent += sent as u64;

                if packets_sent % 10000 == 0 {
                    println!("Sent {} packets", packets_sent);
                }
            }
            Err(e) => {
                println!("Send error: {}", e);
                break;
            }
        }

        // Short delay to limit speed
        thread::sleep(Duration::from_micros(10));
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!("\n====== Test Results ======");
    println!("Total packets sent: {}", packets_sent);
    println!("Total bytes sent: {}", bytes_sent);
    println!("Packets per second: {:.2}", packets_sent as f64 / elapsed);
    println!("Transmission speed: {:.2} Mbps", 8.0 * bytes_sent as f64 / elapsed / 1_000_000.0);
}