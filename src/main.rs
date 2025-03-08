// src/main.rs
use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vma_udp::{VmaOptions, VmaUdpSocket};

const BUFFER_SIZE: usize = 4096;
const TEST_DURATION: u64 = 10; // 테스트 실행 시간(초)

fn main() {
    // 커맨드 라인 인자 파싱
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("사용법: {} [server|client] [ip] [port]", args[0]);
        println!("  기본값: 127.0.0.1:5001");
        process::exit(1);
    }

    let mode = &args[1];
    let ip = args.get(2).map(|s| s.as_str()).unwrap_or("127.0.0.1");
    let port: u16 = args.get(3).map(|s| s.parse().unwrap_or(5001)).unwrap_or(5001);

    // Ctrl-C 핸들링
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("종료 신호 받음, 테스트를 종료합니다...");
    })
    .expect("Error setting Ctrl-C handler");

    match mode.as_str() {
        "server" => run_server(running, ip, port),
        "client" => run_client(running, ip, port),
        _ => {
            println!("알 수 없는 모드: {}", mode);
            println!("'server' 또는 'client'를 사용하세요");
            process::exit(1);
        }
    }
}

fn run_server(running: Arc<AtomicBool>, ip: &str, port: u16) {
    println!("서버 모드 (수신): {}:{}", ip, port);

    // VMA 옵션 설정
    let vma_options = VmaOptions {
        use_socketxtreme: true,
        optimize_for_latency: true,
        use_polling: true,
        ring_count: 4,
        buffer_size: BUFFER_SIZE as i32,
        enable_timestamps: true,
    };

    // UDP 소켓 생성
    let mut socket = match VmaUdpSocket::with_options(vma_options) {
        Ok(s) => s,
        Err(e) => {
            println!("소켓 생성 실패: {}", e);
            return;
        }
    };

    // 주소에 바인딩
    if let Err(e) = socket.bind(ip, port) {
        println!("바인딩 실패: {}", e);
        return;
    }

    println!("UDP 서버가 {}:{} 에서 수신 대기 중", ip, port);

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut packets_received = 0u64;
    let mut bytes_received = 0u64;

    let start_time = std::time::Instant::now();

    // 수신 루프
    while running.load(Ordering::SeqCst) && start_time.elapsed().as_secs() < TEST_DURATION {
        match socket.recv_from(&mut buffer, Some(Duration::from_millis(100))) {
            Ok(Some(packet)) => {
                packets_received += 1;
                bytes_received += packet.data.len() as u64;

                if packets_received % 10000 == 0 {
                    println!("{}개 패킷 수신됨", packets_received);
                }
            }
            Ok(None) => {
                // 타임아웃 - 계속 진행
            }
            Err(e) => {
                println!("수신 오류: {}", e);
                break;
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!("\n====== 테스트 결과 ======");
    println!("총 수신 패킷: {}", packets_received);
    println!("총 수신 바이트: {}", bytes_received);
    println!("초당 패킷: {:.2}", packets_received as f64 / elapsed);
    println!("처리량: {:.2} Mbps", 8.0 * bytes_received as f64 / elapsed / 1_000_000.0);
}

fn run_client(running: Arc<AtomicBool>, ip: &str, port: u16) {
    println!("클라이언트 모드 (송신): {}:{}", ip, port);

    // VMA 옵션 설정
    let vma_options = VmaOptions {
        use_socketxtreme: true,
        optimize_for_latency: true,
        use_polling: true,
        ring_count: 4,
        buffer_size: BUFFER_SIZE as i32,
        enable_timestamps: true,
    };

    // UDP 소켓 생성
    let mut socket = match VmaUdpSocket::with_options(vma_options) {
        Ok(s) => s,
        Err(e) => {
            println!("소켓 생성 실패: {}", e);
            return;
        }
    };

    // 대상 주소 연결
    if let Err(e) = socket.connect(ip, port) {
        println!("연결 실패: {}", e);
        return;
    }

    println!("UDP 클라이언트가 {}:{} 로 데이터 전송 중", ip, port);

    // 테스트 데이터 생성
    let data = vec![b'A'; BUFFER_SIZE];
    let mut packets_sent = 0u64;
    let mut bytes_sent = 0u64;

    let start_time = std::time::Instant::now();

    // 송신 루프
    while running.load(Ordering::SeqCst) && start_time.elapsed().as_secs() < TEST_DURATION {
        match socket.send(&data) {
            Ok(sent) => {
                packets_sent += 1;
                bytes_sent += sent as u64;

                if packets_sent % 10000 == 0 {
                    println!("{}개 패킷 전송됨", packets_sent);
                }
            }
            Err(e) => {
                println!("전송 오류: {}", e);
                break;
            }
        }

        // 속도 제한을 위한 짧은 딜레이
        thread::sleep(Duration::from_micros(10));
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!("\n====== 테스트 결과 ======");
    println!("총 송신 패킷: {}", packets_sent);
    println!("총 송신 바이트: {}", bytes_sent);
    println!("초당 패킷: {:.2}", packets_sent as f64 / elapsed);
    println!("전송 속도: {:.2} Mbps", 8.0 * bytes_sent as f64 / elapsed / 1_000_000.0);
}