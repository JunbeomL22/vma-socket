/**
 * tcp_socket.c - TCP Socket Implementation with VMA Support
 */
#define _POSIX_C_SOURCE 199309L
#define _GNU_SOURCE  // Enable GNU extensions

#include <fcntl.h>
#include <time.h>
#include <sys/time.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <stdint.h>
#include <stdbool.h>
#include <signal.h>
#include <errno.h>
#include <arpa/inet.h>
#include "tcp_socket.h"

// Forward declarations of static functions
static bool would_block(void);
static int wait_for_socket(int fd, bool for_read, int timeout_ms);
static int set_nonblocking(int fd);
static int set_blocking(int fd);
static void setup_vma_env(const tcp_vma_options_t* options);
static void set_default_options(tcp_vma_options_t* options);

// Set VMA environment variables
static void setup_vma_env(const tcp_vma_options_t* options) {
    if (options->use_socketxtreme) {
        setenv("VMA_SOCKETXTREME", "1", 1);
    }
    
    if (options->optimize_for_latency) {
        setenv("VMA_SPEC", "latency", 1);
    }
    
    if (options->use_polling) {
        setenv("VMA_RX_POLL", "1", 1);
        setenv("VMA_SELECT_POLL", "1", 1);
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
    }
}

// Set default VMA options
static void set_default_options(tcp_vma_options_t* options) {
    options->use_socketxtreme = true;
    options->optimize_for_latency = true;
    options->use_polling = true;
    options->ring_count = 4;
    options->buffer_size = 65536;  // 64KB
    options->enable_timestamps = true;
}

// Check if an operation would block
static bool would_block(void) {
    return (errno == EAGAIN || errno == EWOULDBLOCK);
}

// Make a socket non-blocking
static int set_nonblocking(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0) return -1;
    return fcntl(fd, F_SETFL, flags | O_NONBLOCK);
}

// Make a socket blocking
static int set_blocking(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0) return -1;
    return fcntl(fd, F_SETFL, flags & ~O_NONBLOCK);
}

// Wait for socket readiness with timeout
static int wait_for_socket(int fd, bool for_read, int timeout_ms) {
    fd_set fds;
    struct timeval tv;
    
    FD_ZERO(&fds);
    FD_SET(fd, &fds);
    
    if (timeout_ms >= 0) {
        tv.tv_sec = timeout_ms / 1000;
        tv.tv_usec = (timeout_ms % 1000) * 1000;
    }
    
    if (for_read) {
        return select(fd + 1, &fds, NULL, NULL, timeout_ms >= 0 ? &tv : NULL);
    } else {
        return select(fd + 1, NULL, &fds, NULL, timeout_ms >= 0 ? &tv : NULL);
    }
}

tcp_result_t tcp_socket_init(tcp_socket_t* sock, const tcp_vma_options_t* options) {
    if (!sock) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    // Initialize socket structure
    memset(sock, 0, sizeof(tcp_socket_t));
    sock->socket_fd = -1;
    sock->state = TCP_STATE_DISCONNECTED;
    
    // Set options
    if (options) {
        sock->vma_options = *options;
    } else {
        set_default_options(&sock->vma_options);
    }
    
    // Set VMA environment variables
    setup_vma_env(&sock->vma_options);
    
    // Create socket - use the socket() function directly, not through the parameter name
    sock->socket_fd = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (sock->socket_fd < 0) {
        return TCP_ERROR_SOCKET_CREATE;
    }
    
    // Set buffer size
    if (sock->vma_options.buffer_size > 0) {
        int buffer_size = sock->vma_options.buffer_size;
        
        // Set send buffer size
        if (setsockopt(sock->socket_fd, SOL_SOCKET, SO_SNDBUF, 
                    &buffer_size, sizeof(buffer_size)) < 0) {
            close(sock->socket_fd);
            sock->socket_fd = -1;
            return TCP_ERROR_SOCKET_OPTION;
        }
        
        // Set receive buffer size
        if (setsockopt(sock->socket_fd, SOL_SOCKET, SO_RCVBUF, 
                    &buffer_size, sizeof(buffer_size)) < 0) {
            close(sock->socket_fd);
            sock->socket_fd = -1;
            return TCP_ERROR_SOCKET_OPTION;
        }
    }
    
    // Enable TCP keepalive
    int keepalive = 1;
    if (setsockopt(sock->socket_fd, SOL_SOCKET, SO_KEEPALIVE, 
                &keepalive, sizeof(keepalive)) < 0) {
        close(sock->socket_fd);
        sock->socket_fd = -1;
        return TCP_ERROR_SOCKET_OPTION;
    }
    
    // Configure keepalive parameters
    int keepidle = 60;  // Start sending keepalive probes after this many seconds of idle time
    int keepintvl = 10; // Send a keepalive probe every this many seconds
    int keepcnt = 5;    // Number of keepalive probes to send before considering the connection dead
    
    // Set TCP keepalive parameters
    if (setsockopt(sock->socket_fd, IPPROTO_TCP, TCP_KEEPIDLE, 
                &keepidle, sizeof(keepidle)) < 0 ||
        setsockopt(sock->socket_fd, IPPROTO_TCP, TCP_KEEPINTVL, 
                &keepintvl, sizeof(keepintvl)) < 0 ||
        setsockopt(sock->socket_fd, IPPROTO_TCP, TCP_KEEPCNT, 
                &keepcnt, sizeof(keepcnt)) < 0) {
        // Not fatal, just continue
    }
    
    // Set non-blocking if polling is enabled
    if (sock->vma_options.use_polling) {
        if (set_nonblocking(sock->socket_fd) < 0) {
            close(sock->socket_fd);
            sock->socket_fd = -1;
            return TCP_ERROR_SOCKET_OPTION;
        }
    }
    
    // Set TCP nodelay (disable Nagle's algorithm)
    int nodelay = 1;
    if (setsockopt(sock->socket_fd, IPPROTO_TCP, TCP_NODELAY, 
                &nodelay, sizeof(nodelay)) < 0) {
        // Not fatal, just continue
    }
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_close(tcp_socket_t* sock) {
    if (!sock || sock->socket_fd < 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    close(sock->socket_fd);
    sock->socket_fd = -1;
    sock->is_bound = false;
    sock->state = TCP_STATE_DISCONNECTED;
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_bind(tcp_socket_t* sock, const char* ip, uint16_t port) {
    if (!sock || sock->socket_fd < 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    // Set address
    memset(&sock->local_addr, 0, sizeof(sock->local_addr));
    sock->local_addr.sin_family = AF_INET;
    sock->local_addr.sin_port = htons(port);
    
    if (ip) {
        if (inet_pton(AF_INET, ip, &sock->local_addr.sin_addr) <= 0) {
            return TCP_ERROR_INVALID_PARAM;
        }
    } else {
        sock->local_addr.sin_addr.s_addr = INADDR_ANY;
    }
    
    // Allow address reuse
    int reuse = 1;
    if (setsockopt(sock->socket_fd, SOL_SOCKET, SO_REUSEADDR, 
                &reuse, sizeof(reuse)) < 0) {
        return TCP_ERROR_SOCKET_OPTION;
    }
    
    // Bind socket
    if (bind(sock->socket_fd, (struct sockaddr*)&sock->local_addr, 
            sizeof(sock->local_addr)) < 0) {
        return TCP_ERROR_BIND;
    }
    
    sock->is_bound = true;
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_listen(tcp_socket_t* sock, int backlog) {
    if (!sock || sock->socket_fd < 0 || !sock->is_bound) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    if (listen(sock->socket_fd, backlog) < 0) {
        return TCP_ERROR_LISTEN;
    }
    
    sock->state = TCP_STATE_LISTENING;
    sock->backlog = backlog;
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_accept(tcp_socket_t* sock, tcp_client_t* client, int timeout_ms) {
    if (!sock || sock->socket_fd < 0 || !client || sock->state != TCP_STATE_LISTENING) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    // Wait for a connection with timeout
    if (timeout_ms != 0) {
        int select_result = wait_for_socket(sock->socket_fd, true, timeout_ms);
        
        if (select_result == 0) {
            return TCP_ERROR_TIMEOUT;
        } else if (select_result < 0) {
            return TCP_ERROR_ACCEPT;
        }
    }
    
    // Accept connection
    socklen_t addr_len = sizeof(client->addr);
    client->socket_fd = accept(sock->socket_fd, (struct sockaddr*)&client->addr, &addr_len);
    
    if (client->socket_fd < 0) {
        if (would_block()) {
            return TCP_ERROR_TIMEOUT;
        }
        return TCP_ERROR_ACCEPT;
    }
    
    // Initialize client structure
    client->rx_bytes = 0;
    client->tx_bytes = 0;
    
    // Set non-blocking if polling is enabled
    if (sock->vma_options.use_polling) {
        if (set_nonblocking(client->socket_fd) < 0) {
            close(client->socket_fd);
            client->socket_fd = -1;
            return TCP_ERROR_SOCKET_OPTION;
        }
    }
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_connect(tcp_socket_t* sock, const char* ip, uint16_t port, int timeout_ms) {
    if (!sock || sock->socket_fd < 0 || !ip) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    if (sock->state == TCP_STATE_CONNECTED) {
        return TCP_ERROR_ALREADY_CONNECTED;
    }
    
    // Set remote address
    memset(&sock->remote_addr, 0, sizeof(sock->remote_addr));
    sock->remote_addr.sin_family = AF_INET;
    sock->remote_addr.sin_port = htons(port);
    
    if (inet_pton(AF_INET, ip, &sock->remote_addr.sin_addr) <= 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    // We need non-blocking mode for timeout handling
    bool was_nonblocking = sock->vma_options.use_polling;
    if (!was_nonblocking) {
        if (set_nonblocking(sock->socket_fd) < 0) {
            return TCP_ERROR_SOCKET_OPTION;
        }
    }
    
    sock->state = TCP_STATE_CONNECTING;
    
    // Attempt to connect
    int connect_result = connect(sock->socket_fd, (struct sockaddr*)&sock->remote_addr, 
                               sizeof(sock->remote_addr));
    
    if (connect_result < 0) {
        if (errno != EINPROGRESS) {
            sock->state = TCP_STATE_DISCONNECTED;
            if (!was_nonblocking) {
                set_blocking(sock->socket_fd);
            }
            return TCP_ERROR_CONNECT;
        }
        
        // Wait for connection to complete
        int select_result = wait_for_socket(sock->socket_fd, false, timeout_ms);
        
        if (select_result == 0) {
            sock->state = TCP_STATE_DISCONNECTED;
            if (!was_nonblocking) {
                set_blocking(sock->socket_fd);
            }
            return TCP_ERROR_TIMEOUT;
        } else if (select_result < 0) {
            sock->state = TCP_STATE_DISCONNECTED;
            if (!was_nonblocking) {
                set_blocking(sock->socket_fd);
            }
            return TCP_ERROR_CONNECT;
        }
        
        // Check if connection succeeded
        int error;
        socklen_t error_len = sizeof(error);
        if (getsockopt(sock->socket_fd, SOL_SOCKET, SO_ERROR, &error, &error_len) < 0 || error != 0) {
            sock->state = TCP_STATE_DISCONNECTED;
            if (!was_nonblocking) {
                set_blocking(sock->socket_fd);
            }
            return TCP_ERROR_CONNECT;
        }
    }
    
    // Restore socket mode if needed
    if (!was_nonblocking) {
        if (set_blocking(sock->socket_fd) < 0) {
            sock->state = TCP_STATE_DISCONNECTED;
            return TCP_ERROR_SOCKET_OPTION;
        }
    }
    
    sock->state = TCP_STATE_CONNECTED;
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_reconnect(tcp_socket_t* sock, int timeout_ms) {
    if (!sock || sock->socket_fd < 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    // If already connected, nothing to do
    if (sock->state == TCP_STATE_CONNECTED) {
        return TCP_SUCCESS;
    }
    
    // If we don't have connection info, can't reconnect
    if (sock->remote_addr.sin_family == 0) {
        return TCP_ERROR_NOT_INITIALIZED;
    }
    
    // Close existing socket
    close(sock->socket_fd);
    
    // Create a new socket - use the socket() function directly, not via parameter name
    sock->socket_fd = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (sock->socket_fd < 0) {
        sock->state = TCP_STATE_DISCONNECTED;
        return TCP_ERROR_SOCKET_CREATE;
    }
    
    // Set buffer size
    if (sock->vma_options.buffer_size > 0) {
        int buffer_size = sock->vma_options.buffer_size;
        setsockopt(sock->socket_fd, SOL_SOCKET, SO_SNDBUF, &buffer_size, sizeof(buffer_size));
        setsockopt(sock->socket_fd, SOL_SOCKET, SO_RCVBUF, &buffer_size, sizeof(buffer_size));
    }
    
    // Set TCP keepalive
    int keepalive = 1;
    setsockopt(sock->socket_fd, SOL_SOCKET, SO_KEEPALIVE, &keepalive, sizeof(keepalive));
    
    // Set TCP nodelay
    int nodelay = 1;
    setsockopt(sock->socket_fd, IPPROTO_TCP, TCP_NODELAY, &nodelay, sizeof(nodelay));
    
    // Set non-blocking if polling is enabled
    if (sock->vma_options.use_polling) {
        set_nonblocking(sock->socket_fd);
    }
    
    // Try to reconnect
    char ip[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &sock->remote_addr.sin_addr, ip, INET_ADDRSTRLEN);
    uint16_t port = ntohs(sock->remote_addr.sin_port);
    
    tcp_result_t result = tcp_socket_connect(sock, ip, port, timeout_ms);
    
    if (result != TCP_SUCCESS) {
        return TCP_ERROR_RECONNECT;
    }
    
    return TCP_SUCCESS;
}

bool tcp_socket_is_connected(tcp_socket_t* sock) {
    if (!sock || sock->socket_fd < 0) {
        return false;
    }
    
    // Quick check based on state
    if (sock->state != TCP_STATE_CONNECTED) {
        return false;
    }
    
    // More thorough check: try to send 0 bytes
    if (send(sock->socket_fd, NULL, 0, MSG_NOSIGNAL) < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
        sock->state = TCP_STATE_DISCONNECTED;
        return false;
    }
    
    return true;
}

tcp_result_t tcp_socket_send(tcp_socket_t* sock, const void* data, size_t length, size_t* bytes_sent) {
    if (!sock || sock->socket_fd < 0 || !data || length == 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    if (sock->state != TCP_STATE_CONNECTED) {
        return TCP_ERROR_NOT_INITIALIZED;
    }
    
    ssize_t res = send(sock->socket_fd, data, length, MSG_NOSIGNAL);
    
    if (res < 0) {
        if (would_block()) {
            return TCP_ERROR_WOULD_BLOCK;
        }
        sock->state = TCP_STATE_DISCONNECTED;
        return TCP_ERROR_SEND;
    }
    
    if (bytes_sent) {
        *bytes_sent = (size_t)res;
    }
    
    sock->tx_packets++;
    sock->tx_bytes += res;
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_send_to_client(tcp_client_t* client, const void* data, size_t length, size_t* bytes_sent) {
    if (!client || client->socket_fd < 0 || !data || length == 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    ssize_t res = send(client->socket_fd, data, length, MSG_NOSIGNAL);
    
    if (res < 0) {
        if (would_block()) {
            return TCP_ERROR_WOULD_BLOCK;
        }
        return TCP_ERROR_SEND;
    }
    
    if (bytes_sent) {
        *bytes_sent = (size_t)res;
    }
    
    client->tx_bytes += res;
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_recv(tcp_socket_t* sock, void* buffer, size_t buffer_size, 
                            int timeout_ms, size_t* bytes_received) {
    if (!sock || sock->socket_fd < 0 || !buffer || buffer_size == 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    if (sock->state != TCP_STATE_CONNECTED) {
        return TCP_ERROR_NOT_INITIALIZED;
    }
    
    // Handle timeout
    if (timeout_ms != 0) {
        int select_result = wait_for_socket(sock->socket_fd, true, timeout_ms);
        
        if (select_result == 0) {
            return TCP_ERROR_TIMEOUT;
        } else if (select_result < 0) {
            return TCP_ERROR_RECV;
        }
    }
    
    // Receive data
    ssize_t res = recv(sock->socket_fd, buffer, buffer_size, 0);
    
    if (res < 0) {
        if (would_block()) {
            return TCP_ERROR_TIMEOUT;
        }
        sock->state = TCP_STATE_DISCONNECTED;
        return TCP_ERROR_RECV;
    } else if (res == 0) {
        // Connection closed by peer
        sock->state = TCP_STATE_DISCONNECTED;
        return TCP_ERROR_CLOSED;
    }
    
    if (bytes_received) {
        *bytes_received = (size_t)res;
    }
    
    sock->rx_packets++;
    sock->rx_bytes += res;
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_recv_from_client(tcp_client_t* client, void* buffer, size_t buffer_size, 
                                      int timeout_ms, size_t* bytes_received) {
    if (!client || client->socket_fd < 0 || !buffer || buffer_size == 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    // Handle timeout
    if (timeout_ms != 0) {
        int select_result = wait_for_socket(client->socket_fd, true, timeout_ms);
        
        if (select_result == 0) {
            return TCP_ERROR_TIMEOUT;
        } else if (select_result < 0) {
            return TCP_ERROR_RECV;
        }
    }
    
    // Receive data
    ssize_t res = recv(client->socket_fd, buffer, buffer_size, 0);
    
    if (res < 0) {
        if (would_block()) {
            return TCP_ERROR_TIMEOUT;
        }
        return TCP_ERROR_RECV;
    } else if (res == 0) {
        // Connection closed by peer
        return TCP_ERROR_CLOSED;
    }
    
    if (bytes_received) {
        *bytes_received = (size_t)res;
    }
    
    client->rx_bytes += res;
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_close_client(tcp_client_t* client) {
    if (!client || client->socket_fd < 0) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    close(client->socket_fd);
    client->socket_fd = -1;
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_setopt(tcp_socket_t* sock, int level, int optname, 
                            const void* optval, socklen_t optlen) {
    if (!sock || sock->socket_fd < 0 || !optval) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    if (setsockopt(sock->socket_fd, level, optname, optval, optlen) < 0) {
        return TCP_ERROR_SOCKET_OPTION;
    }
    
    return TCP_SUCCESS;
}

tcp_result_t tcp_socket_get_stats(tcp_socket_t* sock, uint64_t* rx_packets, 
                                uint64_t* tx_packets, uint64_t* rx_bytes, 
                                uint64_t* tx_bytes) {
    if (!sock) {
        return TCP_ERROR_INVALID_PARAM;
    }
    
    if (rx_packets) *rx_packets = sock->rx_packets;
    if (tx_packets) *tx_packets = sock->tx_packets;
    if (rx_bytes) *rx_bytes = sock->rx_bytes;
    if (tx_bytes) *tx_bytes = sock->tx_bytes;
    
    return TCP_SUCCESS;
}