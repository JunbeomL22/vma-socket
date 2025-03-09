/**
 * vma_common.c - Common VMA functionality implementation
 */
// #define _GNU_SOURCE  // Enable GNU extensions

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "vma_common.h"

// Set up VMA environment variables based on options
void vma_setup_environment(const vma_options_t* options) {
    if (!options) {
        return;
    }
    
    // Core VMA settings
    if (options->use_socketxtreme) {
        setenv("VMA_SOCKETXTREME", "1", 1);
    }
    
    if (options->optimize_for_latency) {
        setenv("VMA_SPEC", "latency", 1);
    } else {
        // Optimize for throughput
        setenv("VMA_SPEC", "throughput", 1);
    }
    
    if (options->use_polling) {
        setenv("VMA_RX_POLL", "1", 1);
        setenv("VMA_SELECT_POLL", "1", 1);
        
        // Polling optimizations
        if (options->disable_poll_yield) {
            setenv("VMA_RX_POLL_YIELD", "0", 1);
        }
        
        if (options->skip_os_select) {
            setenv("VMA_SELECT_SKIP_OS", "1", 1);
        }
    }
    
    if (options->ring_count > 0) {
        char ring_count[16];
        snprintf(ring_count, sizeof(ring_count), "%d", options->ring_count);
        setenv("VMA_RING_ALLOCATION_LOGIC_RX", ring_count, 1);
    }
    
    // SocketXtreme optimizations
    if (options->use_socketxtreme) {
        setenv("VMA_RING_ALLOCATION_LOGIC_TX", "0", 1);
        setenv("VMA_THREAD_MODE", "1", 1);
        
        if (options->keep_qp_full) {
            setenv("VMA_CQ_KEEP_QP_FULL", "1", 1);
        }
    } else {
        // Multi-threaded mode when not using SocketXtreme
        setenv("VMA_THREAD_MODE", "3", 1);
    }
    
    // Memory optimizations
    if (options->use_hugepages) {
        setenv("VMA_MEMORY_ALLOCATION_TYPE", "2", 1);
    }
    
    // Buffer counts
    if (options->tx_bufs > 0) {
        char tx_bufs[16];
        snprintf(tx_bufs, sizeof(tx_bufs), "%u", options->tx_bufs);
        setenv("VMA_TX_BUFS", tx_bufs, 1);
    }
    
    if (options->rx_bufs > 0) {
        char rx_bufs[16];
        snprintf(rx_bufs, sizeof(rx_bufs), "%u", options->rx_bufs);
        setenv("VMA_RX_BUFS", rx_bufs, 1);
    }
    
    // CPU affinity settings
    if (options->cpu_cores && options->cpu_cores_count > 0) {
        setenv("VMA_THREAD_AFFINITY", "1", 1);
        
        // Create a string like "0,1,2,3"
        size_t str_size = options->cpu_cores_count * 4; // Allow up to 3 digits per core plus comma
        char* cores_str = malloc(str_size);
        if (cores_str) {
            cores_str[0] = '\0';
            int offset = 0;
            
            for (int i = 0; i < options->cpu_cores_count; i++) {
                int written;
                if (i == 0) {
                    written = snprintf(cores_str + offset, str_size - offset, "%d", options->cpu_cores[i]);
                } else {
                    written = snprintf(cores_str + offset, str_size - offset, ",%d", options->cpu_cores[i]);
                }
                
                if (written > 0) {
                    offset += written;
                }
            }
            
            setenv("VMA_THREAD_AFFINITY_ID", cores_str, 1);
            free(cores_str);
        }
    }
    
    // TCP-specific optimizations (always set these as they don't hurt UDP)
    setenv("VMA_TCP_STREAM_RX_SIZE", "16777216", 1); // 16MB
    setenv("VMA_TCP_RX_ZERO_COPY", "1", 1);
    
    // Additional settings from the suggested code change
    if (options->enable_timestamps) {
        setenv("VMA_TIMESTAMP", "1", 1);
    }
}

// Implementation of set_default_options
void set_default_options(vma_options_t* options) {
    if (!options) return;
    
    options->use_socketxtreme = false;
    options->optimize_for_latency = true;
    options->use_polling = false;
    options->ring_count = 1;
    options->buffer_size = 4096;
    options->enable_timestamps = false;
}