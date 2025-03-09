/**
 * udp_socket.h - UDP Socket Structure with VMA Options
 */

#ifndef UDP_SOCKET_H
#define UDP_SOCKET_H

#include <stdint.h>
#include <stdbool.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include "vma_common.h"

// UDP socket structure
typedef struct {
    int socket_fd;                 // Socket file descriptor
    vma_options_t vma_options; // VMA options
    struct sockaddr_in local_addr; // Local address information
    struct sockaddr_in remote_addr; // Remote address information
    bool is_bound;                 // Whether the socket is bound
    bool is_connected;             // Whether the socket is connected (default target set)
    uint64_t rx_packets;           // Number of received packets
    uint64_t tx_packets;           // Number of transmitted packets
    uint64_t rx_bytes;             // Number of received bytes
    uint64_t tx_bytes;             // Number of transmitted bytes
} udp_socket_t;

// Packet structure
typedef struct {
    void* data;                   // Packet data
    size_t length;                // Data length
    struct sockaddr_in src_addr;  // Source address (on receive)
    uint64_t timestamp;           // Timestamp
} udp_packet_t;

// Result codes
typedef enum {
    UDP_SUCCESS = 0,
    UDP_ERROR_SOCKET_CREATE = -1,
    UDP_ERROR_SOCKET_OPTION = -2,
    UDP_ERROR_BIND = -3,
    UDP_ERROR_CONNECT = -4,
    UDP_ERROR_SEND = -5,
    UDP_ERROR_RECV = -6,
    UDP_ERROR_TIMEOUT = -7,
    UDP_ERROR_INVALID_PARAM = -8,
    UDP_ERROR_NOT_INITIALIZED = -9,
    UDP_ERROR_CLOSED = -10
} udp_result_t;

/**
 * Create and initialize a UDP socket
 * 
 * @param socket Pointer to the UDP socket structure to initialize
 * @param options VMA options (use default if NULL)
 * @return Result code
 */
udp_result_t udp_socket_init(udp_socket_t* socket, const vma_options_t* options);

/**
 * Release and close a UDP socket
 * 
 * @param socket Pointer to the UDP socket structure
 * @return Result code
 */
udp_result_t udp_socket_close(udp_socket_t* socket);

/**
 * Bind a UDP socket to a local address
 * 
 * @param socket Pointer to the UDP socket structure
 * @param ip IP address to bind to (use INADDR_ANY if NULL)
 * @param port Port to bind to
 * @return Result code
 */
udp_result_t udp_socket_bind(udp_socket_t* socket, const char* ip, uint16_t port);

/**
 * Set the default target address for a UDP socket (connect)
 * 
 * @param socket Pointer to the UDP socket structure
 * @param ip Target IP address
 * @param port Target port
 * @return Result code
 */
udp_result_t udp_socket_connect(udp_socket_t* socket, const char* ip, uint16_t port);

/**
 * Send data to the default target address
 * 
 * @param socket Pointer to the UDP socket structure
 * @param data Data to send
 * @param length Data length
 * @param bytes_sent Number of bytes sent (can be NULL)
 * @return Result code
 */
udp_result_t udp_socket_send(udp_socket_t* socket, const void* data, size_t length, size_t* bytes_sent);

/**
 * Send data to a specified address
 * 
 * @param socket Pointer to the UDP socket structure
 * @param data Data to send
 * @param length Data length
 * @param ip Target IP address
 * @param port Target port
 * @param bytes_sent Number of bytes sent (can be NULL)
 * @return Result code
 */
udp_result_t udp_socket_sendto(udp_socket_t* socket, const void* data, size_t length, 
                            const char* ip, uint16_t port, size_t* bytes_sent);

/**
 * Receive data
 * 
 * @param socket Pointer to the UDP socket structure
 * @param buffer Receive buffer
 * @param buffer_size Buffer size
 * @param timeout_ms Timeout in milliseconds (0 for non-blocking, -1 for infinite wait)
 * @param bytes_received Number of bytes received (can be NULL)
 * @return Result code
 */
udp_result_t udp_socket_recv(udp_socket_t* socket, void* buffer, size_t buffer_size, 
                            int timeout_ms, size_t* bytes_received);

/**
 * Receive data (including source address information)
 * 
 * @param socket Pointer to the UDP socket structure
 * @param packet Received packet structure
 * @param buffer Receive buffer
 * @param buffer_size Buffer size
 * @param timeout_ms Timeout in milliseconds (0 for non-blocking, -1 for infinite wait)
 * @return Result code
 */
udp_result_t udp_socket_recvfrom(udp_socket_t* socket, udp_packet_t* packet,
                                void* buffer, size_t buffer_size, int timeout_ms);

/**
 * Set socket options
 * 
 * @param socket Pointer to the UDP socket structure
 * @param level Option level (e.g., SOL_SOCKET)
 * @param optname Option name
 * @param optval Option value
 * @param optlen Option value length
 * @return Result code
 */
udp_result_t udp_socket_setopt(udp_socket_t* socket, int level, int optname, 
                            const void* optval, socklen_t optlen);

/**
 * Get socket statistics
 * 
 * @param socket Pointer to the UDP socket structure
 * @param rx_packets Number of received packets (can be NULL)
 * @param tx_packets Number of transmitted packets (can be NULL)
 * @param rx_bytes Number of received bytes (can be NULL)
 * @param tx_bytes Number of transmitted bytes (can be NULL)
 * @return Result code
 */
udp_result_t udp_socket_get_stats(udp_socket_t* socket, uint64_t* rx_packets, 
                                uint64_t* tx_packets, uint64_t* rx_bytes, 
                                uint64_t* tx_bytes);

#endif /* UDP_SOCKET_H */