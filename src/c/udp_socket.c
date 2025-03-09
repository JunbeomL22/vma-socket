/**
 * udp_socket.c - UDP Socket Implementation with VMA Support
 */
#define _POSIX_C_SOURCE 199309L
#define _GNU_SOURCE  // Enable GNU extensions

#include <fcntl.h>
#include <time.h>
#include <sys/time.h>
#include <stdio.h>
#include <stdlib.h>  // Include for setenv
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <stdint.h>
#include <stdbool.h>
#include <signal.h>
#include <errno.h>
#include <arpa/inet.h>  // Include for inet_pton
#include "udp_socket.h"
#include <mellanox/vma_extra.h>

// Enhanced VMA environment setup function with performance optimizations
static void setup_vma_env(const udp_vma_options_t* options) {
    // Original settings
    if (options->use_socketxtreme) {
        setenv("VMA_SOCKETXTREME", "1", 1);
    }
    
    if (options->optimize_for_latency) {
        setenv("VMA_SPEC", "latency", 1);
    }
    
    if (options->use_polling) {
        setenv("VMA_RX_POLL", "1", 1);
        setenv("VMA_SELECT_POLL", "1", 1);
        
        // Add: prevent CPU yielding during polling for lower latency
        setenv("VMA_RX_POLL_YIELD", "0", 1);
        
        // Add: skip OS during select operations for better performance
        setenv("VMA_SELECT_SKIP_OS", "1", 1);
    }
    
    if (options->ring_count > 0) {
        char ring_count[16];
        snprintf(ring_count, sizeof(ring_count), "%d", options->ring_count);
        setenv("VMA_RING_ALLOCATION_LOGIC_RX", ring_count, 1);
    }
    
    // SocketXtreme optimization
    if (options->use_socketxtreme) {
        setenv("VMA_RING_ALLOCATION_LOGIC_TX", "0", 1);
        setenv("VMA_THREAD_MODE", "1", 1);
        
        // Add: Keep queue pairs full for better throughput with SocketXtreme
        setenv("VMA_CQ_KEEP_QP_FULL", "1", 1);
    }
    
    // New optimizations
    
    // Use hugepages for better memory performance
    setenv("VMA_MEMORY_ALLOCATION_TYPE", "2", 1);
    
    // Increase receive and transmit buffer counts 
    setenv("VMA_RX_BUFS", "10000", 1);
    setenv("VMA_TX_BUFS", "10000", 1);
    
    // Enable thread affinity for better CPU cache utilization
    setenv("VMA_THREAD_AFFINITY", "1", 1);
}

// Enhanced UDP socket initialization with additional optimizations
udp_result_t udp_socket_init(udp_socket_t* udp_socket, const udp_vma_options_t* options) {
    if (!udp_socket) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    // Initialize socket structure
    memset(udp_socket, 0, sizeof(udp_socket_t));
    
    // Set options
    if (options) {
        udp_socket->vma_options = *options;
    } else {
        set_default_options(&udp_socket->vma_options);
    }
    
    // Set VMA environment variables
    setup_vma_env(&udp_socket->vma_options);
    
    // Create socket
    udp_socket->socket_fd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (udp_socket->socket_fd < 0) {
        return UDP_ERROR_SOCKET_CREATE;
    }
    
    // Set polling mode
    if (udp_socket->vma_options.use_polling) {
        int flags = fcntl(udp_socket->socket_fd, F_GETFL, 0);
        if (flags >= 0) {
            fcntl(udp_socket->socket_fd, F_SETFL, flags | O_NONBLOCK);
        }
    }
    
    // Set buffer size
    if (udp_socket->vma_options.buffer_size > 0) {
        int buffer_size = udp_socket->vma_options.buffer_size;
        
        // Set send buffer size
        if (setsockopt(udp_socket->socket_fd, SOL_SOCKET, SO_SNDBUF, 
                    &buffer_size, sizeof(buffer_size)) < 0) {
            return UDP_ERROR_SOCKET_OPTION;
        }
        
        // Set receive buffer size
        if (setsockopt(udp_socket->socket_fd, SOL_SOCKET, SO_RCVBUF, 
                    &buffer_size, sizeof(buffer_size)) < 0) {
            return UDP_ERROR_SOCKET_OPTION;
        }
    }
    
    // Enable timestamps if requested
    if (udp_socket->vma_options.enable_timestamps) {
        int optval = 1;
        // Use more precise hardware timestamps when available
        setsockopt(udp_socket->socket_fd, SOL_SOCKET, SO_TIMESTAMPNS, &optval, sizeof(optval));
    }
    
    // Optimize VMA ring allocation when using SocketXtreme
    if (udp_socket->vma_options.use_socketxtreme) {
        int optval = 1;
        setsockopt(udp_socket->socket_fd, SOL_SOCKET, SO_VMA_RING_ALLOC_LOGIC, &optval, sizeof(optval));
    }
    
    return UDP_SUCCESS;
}

udp_result_t udp_socket_close(udp_socket_t* socket) {
    if (!socket || socket->socket_fd < 0) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    close(socket->socket_fd);
    socket->socket_fd = -1;
    socket->is_bound = false;
    socket->is_connected = false;
    
    return UDP_SUCCESS;
}

udp_result_t udp_socket_bind(udp_socket_t* socket, const char* ip, uint16_t port) {
    if (!socket || socket->socket_fd < 0) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    // Set address
    memset(&socket->local_addr, 0, sizeof(socket->local_addr));
    socket->local_addr.sin_family = AF_INET;
    socket->local_addr.sin_port = htons(port);
    
    if (ip) {
        if (inet_pton(AF_INET, ip, &socket->local_addr.sin_addr) <= 0) {
            return UDP_ERROR_INVALID_PARAM;
        }
    } else {
        socket->local_addr.sin_addr.s_addr = INADDR_ANY;
    }
    
    // Bind socket
    if (bind(socket->socket_fd, (struct sockaddr*)&socket->local_addr, 
            sizeof(socket->local_addr)) < 0) {
        return UDP_ERROR_BIND;
    }
    
    socket->is_bound = true;
    return UDP_SUCCESS;
}

udp_result_t udp_socket_connect(udp_socket_t* socket, const char* ip, uint16_t port) {
    if (!socket || socket->socket_fd < 0 || !ip) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    // Set remote address
    memset(&socket->remote_addr, 0, sizeof(socket->remote_addr));
    socket->remote_addr.sin_family = AF_INET;
    socket->remote_addr.sin_port = htons(port);
    
    if (inet_pton(AF_INET, ip, &socket->remote_addr.sin_addr) <= 0) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    // Connect in UDP sets the default target address
    if (connect(socket->socket_fd, (struct sockaddr*)&socket->remote_addr, 
            sizeof(socket->remote_addr)) < 0) {
        return UDP_ERROR_CONNECT;
    }
    
    socket->is_connected = true;
    return UDP_SUCCESS;
}

udp_result_t udp_socket_send(udp_socket_t* socket, const void* data, size_t length, size_t* bytes_sent) {
    if (!socket || socket->socket_fd < 0 || !data || length == 0) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    if (!socket->is_connected) {
        return UDP_ERROR_NOT_INITIALIZED;
    }
    
    ssize_t res = send(socket->socket_fd, data, length, 0);
    
    if (res < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return UDP_ERROR_TIMEOUT;
        }
        return UDP_ERROR_SEND;
    }
    
    if (bytes_sent) {
        *bytes_sent = (size_t)res;
    }
    
    socket->tx_packets++;
    socket->tx_bytes += res;
    
    return UDP_SUCCESS;
}

udp_result_t udp_socket_sendto(udp_socket_t* socket, const void* data, size_t length, 
                            const char* ip, uint16_t port, size_t* bytes_sent) {
    if (!socket || socket->socket_fd < 0 || !data || length == 0 || !ip) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    struct sockaddr_in dest_addr;
    memset(&dest_addr, 0, sizeof(dest_addr));
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons(port);
    
    if (inet_pton(AF_INET, ip, &dest_addr.sin_addr) <= 0) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    ssize_t res = sendto(socket->socket_fd, data, length, 0, 
                    (struct sockaddr*)&dest_addr, sizeof(dest_addr));
    
    if (res < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return UDP_ERROR_TIMEOUT;
        }
        return UDP_ERROR_SEND;
    }
    
    if (bytes_sent) {
        *bytes_sent = (size_t)res;
    }
    
    socket->tx_packets++;
    socket->tx_bytes += res;
    
    return UDP_SUCCESS;
}

udp_result_t udp_socket_recv(udp_socket_t* socket, void* buffer, size_t buffer_size, 
                            int timeout_ms, size_t* bytes_received) {
    if (!socket || socket->socket_fd < 0 || !buffer || buffer_size == 0) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    // Handle timeout
    if (timeout_ms != 0) {
        fd_set readfds;
        struct timeval tv;
        
        FD_ZERO(&readfds);
        FD_SET(socket->socket_fd, &readfds);
        
        if (timeout_ms > 0) {
            tv.tv_sec = timeout_ms / 1000;
            tv.tv_usec = (timeout_ms % 1000) * 1000;
        }
        
        int select_result = select(socket->socket_fd + 1, &readfds, NULL, NULL, 
                                timeout_ms >= 0 ? &tv : NULL);
        
        if (select_result == 0) {
            return UDP_ERROR_TIMEOUT;
        } else if (select_result < 0) {
            return UDP_ERROR_RECV;
        }
    }
    
    // Receive data
    ssize_t res = recv(socket->socket_fd, buffer, buffer_size, 0);
    
    if (res < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return UDP_ERROR_TIMEOUT;
        }
        return UDP_ERROR_RECV;
    } else if (res == 0) {
        return UDP_ERROR_CLOSED;
    }
    
    if (bytes_received) {
        *bytes_received = (size_t)res;
    }
    
    socket->rx_packets++;
    socket->rx_bytes += res;
    
    return UDP_SUCCESS;
}

udp_result_t udp_socket_recvfrom(udp_socket_t* socket, udp_packet_t* packet,
                            void* buffer, size_t buffer_size, int timeout_ms) {
    if (!socket || socket->socket_fd < 0 || !packet || !buffer || buffer_size == 0) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    // Handle timeout
    if (timeout_ms != 0) {
        fd_set readfds;
        struct timeval tv;
        
        FD_ZERO(&readfds);
        FD_SET(socket->socket_fd, &readfds);
        
        if (timeout_ms > 0) {
            tv.tv_sec = timeout_ms / 1000;
            tv.tv_usec = (timeout_ms % 1000) * 1000;
        }
        
        int select_result = select(socket->socket_fd + 1, &readfds, NULL, NULL, 
                                timeout_ms >= 0 ? &tv : NULL);
        
        if (select_result == 0) {
            return UDP_ERROR_TIMEOUT;
        } else if (select_result < 0) {
            return UDP_ERROR_RECV;
        }
    }
    
    // Receive data and address
    socklen_t addr_len = sizeof(packet->src_addr);
    ssize_t res = recvfrom(socket->socket_fd, buffer, buffer_size, 0,
                        (struct sockaddr*)&packet->src_addr, &addr_len);
    
    if (res < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return UDP_ERROR_TIMEOUT;
        }
        return UDP_ERROR_RECV;
    } else if (res == 0) {
        return UDP_ERROR_CLOSED;
    }
    
    // Set packet structure
    packet->data = buffer;
    packet->length = (size_t)res;
    
    // Set timestamp
    struct timespec ts;
    if (clock_gettime(CLOCK_REALTIME, &ts) == 0) {
        packet->timestamp = (uint64_t)ts.tv_sec * 1000000000ULL + ts.tv_nsec;
    } else {
        packet->timestamp = 0;
    }
    
    socket->rx_packets++;
    socket->rx_bytes += res;
    
    return UDP_SUCCESS;
}

udp_result_t udp_socket_setopt(udp_socket_t* socket, int level, int optname, 
                            const void* optval, socklen_t optlen) {
    if (!socket || socket->socket_fd < 0 || !optval) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    if (setsockopt(socket->socket_fd, level, optname, optval, optlen) < 0) {
        return UDP_ERROR_SOCKET_OPTION;
    }
    
    return UDP_SUCCESS;
}

udp_result_t udp_socket_get_stats(udp_socket_t* socket, uint64_t* rx_packets, 
                                uint64_t* tx_packets, uint64_t* rx_bytes, 
                                uint64_t* tx_bytes) {
    if (!socket) {
        return UDP_ERROR_INVALID_PARAM;
    }
    
    if (rx_packets) *rx_packets = socket->rx_packets;
    if (tx_packets) *tx_packets = socket->tx_packets;
    if (rx_bytes) *rx_bytes = socket->rx_bytes;
    if (tx_bytes) *tx_bytes = socket->tx_bytes;
    
    return UDP_SUCCESS;
}