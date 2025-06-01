//! Common types and utilities for VMA socket implementations.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::raw::c_int;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::{self, Visitor};

/// Maximum number of CPU cores that can be specified
const MAX_CPU_CORES: usize = 128;

/// C-compatible VMA options structure that directly matches the C definition.
/// This version is thread-safe by using a fixed-size array instead of raw pointers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmaOptions {
    /// Use SocketXtreme for optimized performance
    pub use_socketxtreme: bool,
    /// Optimize for low latency (may reduce throughput)
    pub optimize_for_latency: bool,
    /// Use polling for faster packet processing
    pub use_polling: bool,
    /// Number of rings to use for polling
    pub ring_count: c_int,
    /// Size of the buffer to use for packet processing
    pub buffer_size: c_int,
    /// Enable packet timestamps for latency measurements
    pub enable_timestamps: bool,
    /// Use hugepages for memory allocation
    pub use_hugepages: bool,
    /// Number of transmit buffers
    pub tx_bufs: u32,
    /// Number of receive buffers
    pub rx_bufs: u32,
    /// Prevent CPU yielding during polling
    pub disable_poll_yield: bool,
    /// Skip OS during select operations
    pub skip_os_select: bool,
    /// Keep queue pairs full for better throughput
    pub keep_qp_full: bool,
    /// CPU cores to use for VMA threads (fixed-size array for thread safety)
    pub cpu_cores: [c_int; MAX_CPU_CORES],
    /// Number of CPU cores in the array
    pub cpu_cores_count: c_int,
}

impl Serialize for VmaOptions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        
        let mut state = serializer.serialize_struct("VmaOptions", 14)?;
        state.serialize_field("use_socketxtreme", &self.use_socketxtreme)?;
        state.serialize_field("optimize_for_latency", &self.optimize_for_latency)?;
        state.serialize_field("use_polling", &self.use_polling)?;
        state.serialize_field("ring_count", &self.ring_count)?;
        state.serialize_field("buffer_size", &self.buffer_size)?;
        state.serialize_field("enable_timestamps", &self.enable_timestamps)?;
        state.serialize_field("use_hugepages", &self.use_hugepages)?;
        state.serialize_field("tx_bufs", &self.tx_bufs)?;
        state.serialize_field("rx_bufs", &self.rx_bufs)?;
        state.serialize_field("disable_poll_yield", &self.disable_poll_yield)?;
        state.serialize_field("skip_os_select", &self.skip_os_select)?;
        state.serialize_field("keep_qp_full", &self.keep_qp_full)?;
        
        // Only serialize the used portion of the array
        let active_cores = &self.cpu_cores[0..self.cpu_cores_count as usize];
        state.serialize_field("cpu_cores", active_cores)?;
        state.serialize_field("cpu_cores_count", &self.cpu_cores_count)?;
        
        state.end()
    }
}

impl<'de> Deserialize<'de> for VmaOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            UseSocketxtreme,
            OptimizeForLatency,
            UsePolling,
            RingCount,
            BufferSize,
            EnableTimestamps,
            UseHugepages,
            TxBufs,
            RxBufs,
            DisablePollYield,
            SkipOsSelect,
            KeepQpFull,
            CpuCores,
            CpuCoresCount,
        }

        struct VmaOptionsVisitor;

        impl<'de> Visitor<'de> for VmaOptionsVisitor {
            type Value = VmaOptions;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct VmaOptions")
            }

            fn visit_map<V>(self, mut map: V) -> Result<VmaOptions, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut options = VmaOptions::default();
                let mut cpu_cores_vec: Option<Vec<c_int>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::UseSocketxtreme => {
                            options.use_socketxtreme = map.next_value()?;
                        }
                        Field::OptimizeForLatency => {
                            options.optimize_for_latency = map.next_value()?;
                        }
                        Field::UsePolling => {
                            options.use_polling = map.next_value()?;
                        }
                        Field::RingCount => {
                            options.ring_count = map.next_value()?;
                        }
                        Field::BufferSize => {
                            options.buffer_size = map.next_value()?;
                        }
                        Field::EnableTimestamps => {
                            options.enable_timestamps = map.next_value()?;
                        }
                        Field::UseHugepages => {
                            options.use_hugepages = map.next_value()?;
                        }
                        Field::TxBufs => {
                            options.tx_bufs = map.next_value()?;
                        }
                        Field::RxBufs => {
                            options.rx_bufs = map.next_value()?;
                        }
                        Field::DisablePollYield => {
                            options.disable_poll_yield = map.next_value()?;
                        }
                        Field::SkipOsSelect => {
                            options.skip_os_select = map.next_value()?;
                        }
                        Field::KeepQpFull => {
                            options.keep_qp_full = map.next_value()?;
                        }
                        Field::CpuCores => {
                            cpu_cores_vec = Some(map.next_value()?);
                        }
                        Field::CpuCoresCount => {
                            options.cpu_cores_count = map.next_value()?;
                        }
                    }
                }

                // Handle CPU cores
                if let Some(cores) = cpu_cores_vec {
                    if cores.len() > MAX_CPU_CORES {
                        return Err(de::Error::custom(format!(
                            "Too many CPU cores: {} > {}",
                            cores.len(),
                            MAX_CPU_CORES
                        )));
                    }
                    
                    options.cpu_cores_count = cores.len() as c_int;
                    for (i, &core) in cores.iter().enumerate() {
                        options.cpu_cores[i] = core;
                    }
                }

                Ok(options)
            }
        }

        const FIELDS: &[&str] = &[
            "use_socketxtreme", "optimize_for_latency", "use_polling", "ring_count",
            "buffer_size", "enable_timestamps", "use_hugepages", "tx_bufs", "rx_bufs",
            "disable_poll_yield", "skip_os_select", "keep_qp_full", "cpu_cores", "cpu_cores_count"
        ];

        deserializer.deserialize_struct("VmaOptions", FIELDS, VmaOptionsVisitor)
    }
}

impl Default for VmaOptions {
    fn default() -> Self {
        VmaOptions {
            use_socketxtreme: true,
            optimize_for_latency: true,
            use_polling: true,
            ring_count: 4,
            buffer_size: 65536, // 64KB
            enable_timestamps: true,
            use_hugepages: true,
            tx_bufs: 10000,
            rx_bufs: 10000,
            disable_poll_yield: true,
            skip_os_select: true,
            keep_qp_full: true,
            cpu_cores: [0; MAX_CPU_CORES],
            cpu_cores_count: 0,
        }
    }
}

impl VmaOptions {
    /// Add a CPU core to the list of cores
    pub fn add_core(&mut self, core: c_int) -> Result<(), &'static str> {
        if self.cpu_cores_count >= MAX_CPU_CORES as c_int {
            return Err("Maximum number of CPU cores reached");
        }
        
        self.cpu_cores[self.cpu_cores_count as usize] = core;
        self.cpu_cores_count += 1;
        Ok(())
    }

    /// Clear all CPU cores
    pub fn clear_cores(&mut self) {
        self.cpu_cores_count = 0;
        // Optional: zero out the array for cleanliness
        self.cpu_cores = [0; MAX_CPU_CORES];
    }

    /// Set multiple CPU cores at once
    pub fn set_cores(&mut self, cores: &[c_int]) -> Result<(), &'static str> {
        if cores.len() > MAX_CPU_CORES {
            return Err("Too many CPU cores specified");
        }
        
        self.clear_cores();
        for &core in cores {
            self.add_core(core)?;
        }
        Ok(())
    }

    /// Get the currently configured CPU cores as a slice
    pub fn get_cores(&self) -> &[c_int] {
        &self.cpu_cores[0..self.cpu_cores_count as usize]
    }

    /// Create options optimized for ultra-low latency
    pub fn low_latency() -> Self {
        VmaOptions {
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
            cpu_cores: [0; MAX_CPU_CORES],
            cpu_cores_count: 0,
        }
    }
    
    /// Create options optimized for high throughput
    pub fn high_throughput() -> Self {
        VmaOptions {
            use_socketxtreme: true,
            optimize_for_latency: false, // Optimize for throughput instead
            use_polling: true,
            ring_count: 4, // Multiple rings for throughput
            buffer_size: 65536, // Larger buffers (64KB)
            enable_timestamps: false, // Disable timestamps for throughput
            use_hugepages: true,
            tx_bufs: 20000, // More buffers for high throughput
            rx_bufs: 20000,
            disable_poll_yield: false, // Allow yielding for throughput
            skip_os_select: true,
            keep_qp_full: true,
            cpu_cores: [0; MAX_CPU_CORES],
            cpu_cores_count: 0,
        }
    }
}

/// Internal representation of socket address in C format.
#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

/// Helper function to convert a Rust Duration to milliseconds for C API calls.
pub fn unixnano_to_ms(duration: Option<u64>) -> c_int {
    match duration {
        Some(t) => (t / 1_000_000) as c_int, // Convert nanoseconds to milliseconds
        None => -1, // wait indefinitely
    }
}

/// Convert a C socket address structure to a Rust SocketAddr.
pub fn sockaddr_to_rust(sockaddr: &SockAddrIn) -> SocketAddr {
    let ip = Ipv4Addr::from(u32::from_be(sockaddr.sin_addr));
    let port = u16::from_be(sockaddr.sin_port);
    SocketAddr::new(IpAddr::V4(ip), port)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_vma_options_serialization() {
        let mut options = VmaOptions::low_latency();
        options.add_core(0).unwrap();
        options.add_core(1).unwrap();
        let serialized = serde_json::to_string_pretty(&options).unwrap();
        let mut file = std::fs::File::create("vma_options.json").unwrap();
        std::io::Write::write_all(&mut file, serialized.as_bytes()).unwrap();
        let deserialized: VmaOptions = serde_json::from_str(&serialized).unwrap();
        println!("Serialized: {}", serialized);
        assert_eq!(options, deserialized);
    }

    #[test]
    fn test_add_core() {
        let mut options = VmaOptions::default();
        assert!(options.add_core(1).is_ok());
        assert_eq!(options.cpu_cores_count, 1);
        assert_eq!(options.cpu_cores[0], 1);
    }

    #[test]
    fn test_set_cores() {
        let mut options = VmaOptions::default();
        assert!(options.set_cores(&[1, 2, 3]).is_ok());
        assert_eq!(options.cpu_cores_count, 3);
        assert_eq!(options.cpu_cores[0], 1);
        assert_eq!(options.cpu_cores[1], 2);
        assert_eq!(options.cpu_cores[2], 3);
    }
}