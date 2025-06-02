use std::env;
use std::process;
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
        "tcp" => benchmark_tcp_send_recv(),
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
        let start_nano = flashlog::get_unix_nano();
        let _ = socket.recv_from(&mut buffer, Some(0)); // 0 = 즉시 리턴
        let end_nano = flashlog::get_unix_nano();
        let duration_nano = end_nano - start_nano;
        recv_times.push(duration_nano);

        if (i + 1) % 10000 == 0 {
            println!("  Completed {}/{} recv iterations", i + 1, ITERATIONS);
        }
    }

    println!("Starting UDP send benchmark ({} iterations)...", ITERATIONS);
    
    // send 벤치마크 (루프백으로 전송)
    let test_data = b"test";
    for i in 0..ITERATIONS {
        let start_nano = flashlog::get_unix_nano();
        let _ = socket.send_to(test_data, "127.0.0.1", 12345); // 아무 포트로 전송
        let end_nano = flashlog::get_unix_nano();
        let duration_nano = end_nano - start_nano;
        send_times.push(duration_nano);

        if (i + 1) % 10000 == 0 {
            println!("  Completed {}/{} send iterations", i + 1, ITERATIONS);
        }
    }

    print_timing_stats(&format!("UDP Recv ({})", config_name), &recv_times);
    print_timing_stats(&format!("UDP Send ({})", config_name), &send_times);
}

fn benchmark_tcp_empty_recv() {
    println!("=== TCP Empty Socket Polling Benchmark ===");
    
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
    
    // 소켓 생성
    let mut socket = match VmaTcpSocket::with_options(options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create TCP socket: {}", e);
            return;
        }
    };

    // 포트에 바인드
    if let Err(e) = socket.bind("127.0.0.1", 0) { // 0 = 자동 포트 할당
        println!("Failed to bind TCP socket: {}", e);
        return;
    }

    // 리스닝 시작
    if let Err(e) = socket.listen(1) {
        println!("Failed to listen on TCP socket: {}", e);
        return;
    }

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut accept_times = Vec::with_capacity(ITERATIONS);
    let mut recv_times = Vec::with_capacity(ITERATIONS);

    println!("Warming up...");
    // 워밍업
    for _ in 0..1000 {
        let _ = socket.accept(Some(0)); // 0 = 논블로킹
    }

    println!("Starting TCP accept benchmark ({} iterations)...", ITERATIONS);
    
    // accept 벤치마크 (빈 소켓 폴링)
    for i in 0..ITERATIONS {
        let start = Instant::now();
        let _ = socket.accept(Some(0)); // 0 = 즉시 리턴
        let duration = start.elapsed();
        accept_times.push(duration);

        if (i + 1) % 10000 == 0 {
            println!("  Completed {}/{} accept iterations", i + 1, ITERATIONS);
        }
    }

    // 연결된 소켓이 없으므로 recv는 테스트하지 않음 (의미가 없음)
    
    print_timing_stats(&format!("TCP Accept ({})", config_name), &accept_times);
    
    // TCP는 연결이 필요하므로 별도의 연결 테스트도 해보자
    benchmark_tcp_connect_timing(config_name, options);
}

fn benchmark_tcp_connect_timing(config_name: &str, mut options: VmaOptions) {
    println!("Starting TCP connect timing test...");
    
    // 클라이언트 소켓 생성
    let mut client_socket = match VmaTcpSocket::with_options(options) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to create TCP client socket: {}", e);
            return;
        }
    };

    let mut connect_times = Vec::with_capacity(1000);
    
    // 존재하지 않는 포트로 연결 시도 (즉시 실패)
    for i in 0..1000 {
        let start_nano = flashlog::get_unix_nano();
        let _ = client_socket.connect("127.0.0.1", 12345, Some(0)); // 0 = 즉시 리턴
        let end_nano = flashlog::get_unix_nano();
        let duration_nano = end_nano - start_nano;
        connect_times.push(duration_nano);

        if (i + 1) % 100 == 0 {
            println!("  Completed {}/{} connect attempts", i + 1, 1000);
        }
    }

    print_timing_stats(&format!("TCP Connect Attempt ({})", config_name), &connect_times);
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