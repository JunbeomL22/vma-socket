/**
 * vma_common.h - Common VMA functionality shared between TCP and UDP
 */

#ifndef VMA_COMMON_H
#define VMA_COMMON_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>  
#include <stdio.h>

// VMA options structure to be shared between TCP and UDP
typedef struct {
    bool use_socketxtreme;       // Whether to use SocketXtreme mode
    bool optimize_for_latency;   // Whether to optimize for latency (useful for real-time applications)
    bool use_polling;            // Whether to use polling mode (reduces latency at the cost of higher CPU usage)
    bool non_blocking;           // Whether to use non-blocking mode
    int ring_count;              // Number of rings (used for load balancing and performance optimization)
    int buffer_size;             // Default buffer size
    bool enable_timestamps;      // Whether to enable timestamps
    bool use_hugepages;          // Whether to use hugepages for memory allocation
    uint32_t tx_bufs;            // Number of transmit buffers
    uint32_t rx_bufs;            // Number of receive buffers
    bool disable_poll_yield;     // Prevent CPU yielding during polling
    bool skip_os_select;         // Skip OS during select operations
    bool keep_qp_full;           // Keep queue pairs full for better throughput
    int* cpu_cores;              // Array of CPU cores to use for affinity
    int cpu_cores_count;         // Number of CPU cores in the array
} vma_options_t;

/**
 * Set up VMA environment variables based on options
 * 
 * @param options VMA options structure
 */
void vma_setup_environment(const vma_options_t* options);

/**
 * Set default VMA options
 * 
 * @param options VMA options structure
 */
void set_default_options(vma_options_t* options);

#endif /* VMA_COMMON_H */