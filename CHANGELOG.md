# 0.1.0
 - date: 2025-03-09
    - change: Initial release
    
# 0.1.1
 - date: 2025-03-09
    - change: documentation: Update README.md and doc comments
# 0.1.2
 - date: 2025-03-09
    - change: documentation: Cargo.toml documentation

# 0.1.3
 - date :2025-05-25
   - change: Add serialization and deserialization for `VmaOptions`
   - `cpu_cores: *mut c_int -> [c_int; MAX_CPU_CORES]`
   - timeout input in recv and send: `Option<Duration> -> Option<u64>` (nanoseconds)

# 0.1.4
 - date : 2025-06-01
   - change Error type from `String` to `std::io::Error`

# 0.1.5
 - date : under dev
   - added benchmark
   - implemented `Debug` and `Clone`
   - build include `/usr/include` and `/usr/include/mellanox`
   - `run.sh` header changed to `#!/usr/bin/bash`