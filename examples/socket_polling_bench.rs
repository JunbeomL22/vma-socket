use std::env;
use std::process;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc;
use vma_socket::udp::VmaUdpSocket;
use vma_socket::tcp::VmaTcpSocket;
use vma_socket::common::VmaOptions;

const ITERATIONS: usize = 100_000;
const BUFFER_SIZE: usize = 1024;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} [udp|tcp]", args[0]);
        process::exit(1);
    }

    let protocol = &args[1];

    match protocol.as_str() {
        "udp" => benchmark_udp_empty_recv(),
        "tcp" => benchmark_tcp_recv(),
        _ => {
            println!("Unknown protocol: {}", protocol);
            println!("Use 'udp' or 'tcp'");
            process::exit(1);
        }
    }
}

fn benchmark_udp_empty_recv() {
    println!("=== UDP Empty Socket Polling Benchmark ===");
    
    // 두 가지 설정으로 테스트
    benchmark_udp_with_options("Low Latency (Polling)", VmaOptions::low_latency());
    benchmark_udp_with_options("High Throughput", VmaOptions::high_throughput());
    
    // 기본 설정
    let mut default_options = VmaOptions::default();
    default_options.use_polling = false; // 논폴링 모드
    benchmark_udp_with_options("Default (Non-Polling)", default_options);
}

fn benchmark_udp_with_options(config_name: &str, mut options: VmaOptions) {
    println!("\n--- {} Configuration ---", config_name);
    
    // CPU 코어 설정
    options.add_core(0).expect("Failed to set CPU core");
    
    // 소켓 생성
    let mut socket = match VmaUdpSocket::with_options(options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create UDP socket: {}", e);
            return;
        }
    };

    // 포트에 바인드 (필수)
    if let Err(e) = socket.bind("127.0.0.1", 0) { // 0 = 자동 포트 할당
        println!("Failed to bind UDP socket: {}", e);
        return;
    }

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut recv_times = Vec::with_capacity(ITERATIONS);
    let mut send_times = Vec::with_capacity(ITERATIONS);

    println!("Warming up...");
    // 워밍업
    for _ in 0..1000 {
        let _ = socket.recv_from(&mut buffer, Some(0)); // 0 = 논블로킹
    }

    println!("Starting UDP recv benchmark ({} iterations)...", ITERATIONS);
    
    // recv 벤치마크 (빈 소켓 폴링)
    for i in 0..ITERATIONS {
        let start = Instant::now();
        let _ = socket.recv_from(&mut buffer, Some(0)); // 0 = 즉시 리턴
        let duration = start.elapsed();
        recv_times.push(duration.as_nanos() as u64);

        if (i + 1) % 10000 == 0 {
            println!("  Completed {}/{} recv iterations", i + 1, ITERATIONS);
        }
    }

    println!("Starting UDP send benchmark ({} iterations)...", ITERATIONS);
    
    // send 벤치마크 (루프백으로 전송)
    let test_data = b"test";
    for i in 0..ITERATIONS {
        let start = Instant::now();
        let _ = socket.send_to(test_data, "127.0.0.1", 12345); // 아무 포트로 전송
        let duration = start.elapsed();
        send_times.push(duration.as_nanos() as u64);

        if (i + 1) % 10000 == 0 {
            println!("  Completed {}/{} send iterations", i + 1, ITERATIONS);
        }
    }

    print_timing_stats(&format!("UDP Recv ({})", config_name), &recv_times);
    print_timing_stats(&format!("UDP Send ({})", config_name), &send_times);
}

fn benchmark_tcp_recv() {
    println!("=== TCP Recv Benchmark ===");
    
    // 두 가지 설정으로 테스트
    benchmark_tcp_with_options("Low Latency (Polling)", VmaOptions::low_latency());
    benchmark_tcp_with_options("High Throughput", VmaOptions::high_throughput());
    
    // 기본 설정
    let mut default_options = VmaOptions::default();
    default_options.use_polling = false; // 논폴링 모드
    benchmark_tcp_with_options("Default (Non-Polling)", default_options);
}

fn benchmark_tcp_with_options(config_name: &str, mut options: VmaOptions) {
    println!("\n--- {} Configuration ---", config_name);
    
    // CPU 코어 설정
    options.add_core(0).expect("Failed to set CPU core");
    
    // 채널을 사용해서 서버와 클라이언트 동기화
    let (tx, rx) = mpsc::channel();
    let options_clone = options.clone();
    
    // 서버 스레드 시작
    let server_handle = thread::spawn(move || {
        run_tcp_server(config_name, options_clone, tx);
    });
    
    // 서버가 준비될 때까지 대기
    let server_port = rx.recv().expect("Failed to receive server port");
    
    // 클라이언트 실행
    run_tcp_client(config_name, options, server_port);
    
    // 서버 스레드 종료 대기
    server_handle.join().expect("Server thread panicked");
}

fn run_tcp_server(config_name: &str, mut options: VmaOptions, tx: mpsc::Sender<u16>) {
    // 서버 소켓 생성
    let mut server_socket = match VmaTcpSocket::with_options(options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create TCP server socket: {}", e);
            return;
        }
    };

    // 포트에 바인드
    if let Err(e) = server_socket.bind("127.0.0.1", 0) { // 0 = 자동 포트 할당
        println!("Failed to bind TCP server socket: {}", e);
        return;
    }

    // 리스닝 시작
    if let Err(e) = server_socket.listen(1) {
        println!("Failed to listen on TCP server socket: {}", e);
        return;
    }

    // 사용된 포트 번호를 클라이언트에게 전달 (실제로는 getsockname으로 가져와야 하지만 여기서는 고정 포트 사용)
    tx.send(8080).expect("Failed to send server port");

    println!("TCP Server waiting for connection...");
    
    // 클라이언트 연결 대기
    match server_socket.accept(Some(5_000_000_000)) { // 5초 타임아웃
        Ok(Some(mut client)) => {
            println!("Client connected from {}", client.address);
            
            let mut buffer = vec![0u8; BUFFER_SIZE];
            let mut recv_times = Vec::with_capacity(ITERATIONS);

            println!("Warming up TCP recv...");
            // 워밍업
            for _ in 0..1000 {
                let _ = client.recv(&mut buffer, Some(0)); // 0 = 논블로킹
            }

            println!("Starting TCP recv benchmark ({} iterations)...", ITERATIONS);
            
            // recv 벤치마크 (빈 소켓 폴링)
            for i in 0..ITERATIONS {
                let start = Instant::now();
                let _ = client.recv(&mut buffer, Some(0)); // 0 = 즉시 리턴
                let duration = start.elapsed();
                recv_times.push(duration.as_nanos() as u64);

                if (i + 1) % 10000 == 0 {
                    println!("  Completed {}/{} recv iterations", i + 1, ITERATIONS);
                }
            }

            print_timing_stats(&format!("TCP Recv ({})", config_name), &recv_times);
        },
        Ok(None) => {
            println!("No client connection within timeout");
        },
        Err(e) => {
            println!("Failed to accept client: {}", e);
        }
    }
}

fn run_tcp_client(config_name: &str, mut options: VmaOptions, server_port: u16) {
    // 잠깐 대기해서 서버가 listen 상태가 되도록 함
    thread::sleep(Duration::from_millis(100));
    
    // 클라이언트 소켓 생성
    let mut client_socket = match VmaTcpSocket::with_options(options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create TCP client socket: {}", e);
            return;
        }
    };

    println!("TCP Client connecting to server...");
    
    // 서버에 연결
    match client_socket.connect("127.0.0.1", server_port, Some(5_000_000_000)) { // 5초 타임아웃
        Ok(true) => {
            println!("Connected to server");
            
            // 연결 유지를 위해 잠시 대기
            thread::sleep(Duration::from_secs(2));
            
            // send 벤치마크도 같이 수행
            let test_data = b"test";
            let mut send_times = Vec::with_capacity(1000);

            println!("Starting TCP send benchmark (1000 iterations)...");
            
            for i in 0..1000 {
                let start = Instant::now();
                let _ = client_socket.send(test_data);
                let duration = start.elapsed();
                send_times.push(duration.as_nanos() as u64);

                if (i + 1) % 100 == 0 {
                    println!("  Completed {}/{} send iterations", i + 1, 1000);
                }
                
                // 너무 빠르게 보내지 않도록 약간의 지연
                thread::sleep(Duration::from_micros(10));
            }

            print_timing_stats(&format!("TCP Send ({})", config_name), &send_times);
        },
        Ok(false) => {
            println!("Connection timeout");
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
}

fn print_timing_stats(operation: &str, times: &[u64]) {
    if times.is_empty() {
        println!("{}: No data", operation);
        return;
    }

    let mut sorted_times = times.to_vec();
    sorted_times.sort();

    let count = sorted_times.len();
    let total_nanos: u64 = sorted_times.iter().sum();
    
    let min_ns = sorted_times[0] as f64;
    let max_ns = sorted_times[count - 1] as f64;
    let avg_ns = (total_nanos as f64) / (count as f64);
    
    let p50_ns = sorted_times[count * 50 / 100] as f64;
    let p95_ns = sorted_times[count * 95 / 100] as f64;
    let p99_ns = sorted_times[count * 99 / 100] as f64;

    println!("\n{} Statistics ({} samples):", operation, count);
    println!("  Min:     {:.0} ns ({:.2} μs)", min_ns, min_ns / 1000.0);
    println!("  Average: {:.0} ns ({:.2} μs)", avg_ns, avg_ns / 1000.0);
    println!("  P50:     {:.0} ns ({:.2} μs)", p50_ns, p50_ns / 1000.0);
    println!("  P95:     {:.0} ns ({:.2} μs)", p95_ns, p95_ns / 1000.0);
    println!("  P99:     {:.0} ns ({:.2} μs)", p99_ns, p99_ns / 1000.0);
    println!("  Max:     {:.0} ns ({:.2} μs)", max_ns, max_ns / 1000.0);
    
    // 처리량 계산
    let ops_per_sec = 1_000_000_000.0 / avg_ns;
    println!("  Throughput: {:.0} ops/sec", ops_per_sec);
}