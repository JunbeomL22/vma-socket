use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vma_socket::tcp::{VmaTcpSocket, Client};
use vma_socket::common::VmaOptions;

const BUFFER_SIZE: usize = 4096;
const TEST_DURATION: u64 = 10; // Test duration in seconds

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} [server|client] [ip] [port]", args[0]);
        println!("  Default: 127.0.0.1:5002");
        process::exit(1);
    }

    let mode = &args[1];
    let ip = args.get(2).map(|s| s.as_str()).unwrap_or("127.0.0.1");
    let port: u16 = args.get(3).map(|s| s.parse().unwrap_or(5002)).unwrap_or(5002);

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
    println!("TCP Server mode (receiving): {}:{}", ip, port);

    // Set VMA options
    let vma_options = VmaOptions::low_latency();

    // Create TCP socket
    let mut socket = match VmaTcpSocket::with_options(vma_options) {
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

    // Listen for connections
    if let Err(e) = socket.listen(10) {
        println!("Failed to listen: {}", e);
        return;
    }

    println!("TCP server listening on {}:{}", ip, port);

    let mut client_opt: Option<Client> = None;
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut bytes_received = 0u64;

    let start_time = std::time::Instant::now();

    // Main server loop
    while running.load(Ordering::SeqCst) && start_time.elapsed().as_secs() < TEST_DURATION {
        // Accept connection if none
        if client_opt.is_none() {
            match socket.accept(Some(Duration::from_millis(100))) {
                Ok(Some(client)) => {
                    println!("Client connected from {}", client.address);
                    client_opt = Some(client);
                }
                Ok(None) => {
                    // Timeout - continue
                }
                Err(e) => {
                    println!("Accept error: {}", e);
                    break;
                }
            }
        }

        // Read from client if connected
        if let Some(ref mut client) = client_opt {
            match client.recv(&mut buffer, Some(Duration::from_millis(100))) {
                Ok(0) => {
                    // No data or connection closed
                    if start_time.elapsed().as_secs() % 2 == 0 {
                        println!("Waiting for data...");
                    }
                }
                Ok(len) => {
                    bytes_received += len as u64;
                    if bytes_received % (1024 * 1024) < BUFFER_SIZE as u64 {
                        println!("Received {} MB", bytes_received / (1024 * 1024));
                    }
                }
                Err(e) => {
                    println!("Client read error: {:?}", e);
                    client_opt = None;
                }
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!("\n====== Test Results ======");
    println!("Total bytes received: {}", bytes_received);
    println!("Throughput: {:.2} Mbps", 8.0 * bytes_received as f64 / elapsed / 1_000_000.0);
}

fn run_client(running: Arc<AtomicBool>, ip: &str, port: u16) {
    println!("TCP Client mode (sending): {}:{}", ip, port);

    // Set VMA options
    let vma_options = VmaOptions::low_latency();

    // Create TCP socket
    let mut socket = match VmaTcpSocket::with_options(vma_options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create socket: {}", e);
            return;
        }
    };

    // Connect to server
    println!("Connecting to {}:{}...", ip, port);
    match socket.connect(ip, port, Some(Duration::from_secs(5))) {
        Ok(true) => println!("Connected to server"),
        Ok(false) => {
            println!("Connection timeout");
            return;
        }
        Err(e) => {
            println!("Failed to connect: {}", e);
            return;
        }
    }

    // Create test data
    let data = vec![b'A'; BUFFER_SIZE];
    let mut bytes_sent = 0u64;

    let start_time = std::time::Instant::now();

    // Main sending loop
    while running.load(Ordering::SeqCst) 
          && start_time.elapsed().as_secs() < TEST_DURATION 
          && socket.is_connected() {
        
        match socket.send(&data) {
            Ok(sent) => {
                if sent > 0 {
                    bytes_sent += sent as u64;
                    if bytes_sent % (1024 * 1024) < BUFFER_SIZE as u64 {
                        println!("Sent {} MB", bytes_sent / (1024 * 1024));
                    }
                } else {
                    // Would block, small delay
                    thread::sleep(Duration::from_micros(10));
                }
            }
            Err(e) => {
                println!("Send error: {}", e);
                // Try to reconnect
                println!("Trying to reconnect...");
                if let Ok(true) = socket.try_reconnect(Some(Duration::from_secs(1))) {
                    println!("Reconnected");
                } else {
                    println!("Reconnect failed");
                    break;
                }
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!("\n====== Test Results ======");
    println!("Total bytes sent: {}", bytes_sent);
    println!("Throughput: {:.2} Mbps", 8.0 * bytes_sent as f64 / elapsed / 1_000_000.0);
}