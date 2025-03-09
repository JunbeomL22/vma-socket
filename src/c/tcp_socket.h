/**
 * tcp_socket.h - TCP Socket Structure with VMA Options
 */

#ifndef TCP_SOCKET_H
#define TCP_SOCKET_H

#include <stdint.h>
#include <stdbool.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include "vma_common.h"

// TCP connection state
typedef enum {
    TCP_STATE_DISCONNECTED = 0,
    TCP_STATE_CONNECTING = 1,
    TCP_STATE_CONNECTED = 2,
    TCP_STATE_LISTENING = 3
} tcp_connection_state_t;

// TCP socket structure
typedef struct {
    int socket_fd;                  // Socket file descriptor
    vma_options_t vma_options;  // VMA options
    struct sockaddr_in local_addr;  // Local address information
    struct sockaddr_in remote_addr; // Remote address information
    bool is_bound;                  // Whether the socket is bound
    tcp_connection_state_t state;   // Connection state
    uint64_t rx_packets;            // Number of received packets
    uint64_t tx_packets;            // Number of transmitted packets
    uint64_t rx_bytes;              // Number of received bytes
    uint64_t tx_bytes;              // Number of transmitted bytes
    int backlog;                    // Listen backlog
} tcp_socket_t;

// Client info structure (for accepted connections)
typedef struct {
    int socket_fd;                  // Client socket file descriptor
    struct sockaddr_in addr;        // Client address
    uint64_t rx_bytes;              // Bytes received from this client
    uint64_t tx_bytes;              // Bytes sent to this client
} tcp_client_t;

// Result codes
typedef enum {
    TCP_SUCCESS = 0,
    TCP_ERROR_SOCKET_CREATE = -1,
    TCP_ERROR_SOCKET_OPTION = -2,
    TCP_ERROR_BIND = -3,
    TCP_ERROR_LISTEN = -4,
    TCP_ERROR_ACCEPT = -5,
    TCP_ERROR_CONNECT = -6,
    TCP_ERROR_RECONNECT = -7,
    TCP_ERROR_SEND = -8,
    TCP_ERROR_RECV = -9,
    TCP_ERROR_TIMEOUT = -10,
    TCP_ERROR_INVALID_PARAM = -11,
    TCP_ERROR_NOT_INITIALIZED = -12,
    TCP_ERROR_CLOSED = -13,
    TCP_ERROR_WOULD_BLOCK = -14,
    TCP_ERROR_ALREADY_CONNECTED = -15
} tcp_result_t;

/**
 * Create and initialize a TCP socket
 * 
 * @param socket Pointer to the TCP socket structure to initialize
 * @param options VMA options (use default if NULL)
 * @return Result code
 */
tcp_result_t tcp_socket_init(tcp_socket_t* socket, const vma_options_t* options);

/**
 * Release and close a TCP socket
 * 
 * @param socket Pointer to the TCP socket structure
 * @return Result code
 */
tcp_result_t tcp_socket_close(tcp_socket_t* socket);

/**
 * Bind a TCP socket to a local address
 * 
 * @param socket Pointer to the TCP socket structure
 * @param ip IP address to bind to (use INADDR_ANY if NULL)
 * @param port Port to bind to
 * @return Result code
 */
tcp_result_t tcp_socket_bind(tcp_socket_t* socket, const char* ip, uint16_t port);

/**
 * Put the socket in listening mode (server)
 * 
 * @param socket Pointer to the TCP socket structure
 * @param backlog Maximum length of the pending connections queue
 * @return Result code
 */
tcp_result_t tcp_socket_listen(tcp_socket_t* socket, int backlog);

/**
 * Accept a new client connection (server)
 * 
 * @param socket Pointer to the TCP socket structure
 * @param client Output pointer to store client information
 * @param timeout_ms Timeout in milliseconds (0 for non-blocking, -1 for infinite wait)
 * @return Result code
 */
tcp_result_t tcp_socket_accept(tcp_socket_t* socket, tcp_client_t* client, int timeout_ms);

/**
 * Connect to a server (client)
 * 
 * @param socket Pointer to the TCP socket structure
 * @param ip Target IP address
 * @param port Target port
 * @param timeout_ms Timeout in milliseconds (0 for non-blocking, -1 for infinite wait)
 * @return Result code
 */
tcp_result_t tcp_socket_connect(tcp_socket_t* socket, const char* ip, uint16_t port, int timeout_ms);

/**
 * Attempt to reconnect (when connection was lost)
 * 
 * @param socket Pointer to the TCP socket structure
 * @param timeout_ms Timeout in milliseconds (0 for non-blocking, -1 for infinite wait)
 * @return Result code
 */
tcp_result_t tcp_socket_reconnect(tcp_socket_t* socket, int timeout_ms);

/**
 * Check if the connection is still alive
 * 
 * @param socket Pointer to the TCP socket structure
 * @return True if connected, false otherwise
 */
bool tcp_socket_is_connected(tcp_socket_t* socket);

/**
 * Send data
 * 
 * @param socket Pointer to the TCP socket structure
 * @param data Data to send
 * @param length Data length
 * @param bytes_sent Number of bytes sent (can be NULL)
 * @return Result code
 */
tcp_result_t tcp_socket_send(tcp_socket_t* socket, const void* data, size_t length, size_t* bytes_sent);

/**
 * Send data on a client socket
 * 
 * @param client Pointer to the client structure
 * @param data Data to send
 * @param length Data length
 * @param bytes_sent Number of bytes sent (can be NULL)
 * @return Result code
 */
tcp_result_t tcp_socket_send_to_client(tcp_client_t* client, const void* data, size_t length, size_t* bytes_sent);

/**
 * Receive data
 * 
 * @param socket Pointer to the TCP socket structure
 * @param buffer Receive buffer
 * @param buffer_size Buffer size
 * @param timeout_ms Timeout in milliseconds (0 for non-blocking, -1 for infinite wait)
 * @param bytes_received Number of bytes received (can be NULL)
 * @return Result code
 */
tcp_result_t tcp_socket_recv(tcp_socket_t* socket, void* buffer, size_t buffer_size, 
                            int timeout_ms, size_t* bytes_received);

/**
 * Receive data from a client
 * 
 * @param client Pointer to the client structure
 * @param buffer Receive buffer
 * @param buffer_size Buffer size
 * @param timeout_ms Timeout in milliseconds (0 for non-blocking, -1 for infinite wait)
 * @param bytes_received Number of bytes received (can be NULL)
 * @return Result code
 */
tcp_result_t tcp_socket_recv_from_client(tcp_client_t* client, void* buffer, size_t buffer_size, 
                                    int timeout_ms, size_t* bytes_received);

/**
 * Close a client connection
 * 
 * @param client Pointer to the client structure
 * @return Result code
 */
tcp_result_t tcp_socket_close_client(tcp_client_t* client);

/**
 * Set socket options
 * 
 * @param socket Pointer to the TCP socket structure
 * @param level Option level (e.g., SOL_SOCKET)
 * @param optname Option name
 * @param optval Option value
 * @param optlen Option value length
 * @return Result code
 */
tcp_result_t tcp_socket_setopt(tcp_socket_t* socket, int level, int optname, 
                            const void* optval, socklen_t optlen);

/**
 * Get socket statistics
 * 
 * @param socket Pointer to the TCP socket structure
 * @param rx_packets Number of received packets (can be NULL)
 * @param tx_packets Number of transmitted packets (can be NULL)
 * @param rx_bytes Number of received bytes (can be NULL)
 * @param tx_bytes Number of transmitted bytes (can be NULL)
 * @return Result code
 */
tcp_result_t tcp_socket_get_stats(tcp_socket_t* socket, uint64_t* rx_packets, 
                                uint64_t* tx_packets, uint64_t* rx_bytes, 
                                uint64_t* tx_bytes);

#endif /* TCP_SOCKET_H */