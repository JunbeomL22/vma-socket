/**
 * tcp_test.c - Example of using TCP Socket structure with VMA
 * 
 * Compile: gcc -o tcp_test tcp_test.c tcp_socket.c -pthread
 * Run: LD_PRELOAD=/usr/lib64/libvma.so.9.8.51 ./tcp_test [server|client] [ip] [port]
 **/

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <stdint.h>
#include <stdbool.h>
#include <signal.h>
#include <sys/time.h>
#include <arpa/inet.h>
#include "tcp_socket.h"

#define BUFFER_SIZE 4096
#define TEST_DURATION 10  // Test duration (seconds)
#define DEFAULT_PORT 5002

// Global variables
volatile int running = 1;
uint64_t bytes_sent = 0;
uint64_t bytes_received = 0;

// Signal handler for termination
void signal_handler(int sig) {
    running = 0;
    printf("\nReceived termination signal, ending test...\n");
}

// Structure to hold client data for server threads
typedef struct {
    tcp_client_t client;
    volatile int* running;
} client_thread_data_t;

// Client thread function (for server mode)
void* client_handler_thread(void* arg) {
    client_thread_data_t* data = (client_thread_data_t*)arg;
    tcp_client_t* client = &data->client;
    volatile int* running = data->running;
    
    char ip_str[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &client->addr.sin_addr, ip_str, INET_ADDRSTRLEN);
    printf("Client thread started for %s:%d\n", ip_str, ntohs(client->addr.sin_port));
    
    // Receive buffer
    char buffer[BUFFER_SIZE];
    
    // Processing loop
    while (*running) {
        size_t bytes_received_now = 0;
        tcp_result_t result = tcp_socket_recv_from_client(
            client, 
            buffer, 
            BUFFER_SIZE, 
            100, // 100ms timeout
            &bytes_received_now
        );
        
        if (result == TCP_SUCCESS) {
            bytes_received += bytes_received_now;
            
            // Print status every 1MB
            if ((bytes_received / (1024 * 1024)) != ((bytes_received - bytes_received_now) / (1024 * 1024))) {
                printf("Received %lu MB\n", bytes_received / (1024 * 1024));
            }
        } else if (result == TCP_ERROR_TIMEOUT) {
            // Timeout - just continue
        } else if (result == TCP_ERROR_CLOSED) {
            printf("Client disconnected\n");
            break;
        } else {
            printf("Error receiving from client: %d\n", result);
            break;
        }
    }
    
    // Close client connection
    tcp_socket_close_client(client);
    
    printf("Client thread ended: %lu bytes received\n", bytes_received);
    free(data);
    return NULL;
}

// Server function
void run_server(const char* ip, uint16_t port) {
    // Initialize TCP socket
    tcp_socket_t server;
    tcp_vma_options_t options = {
        .use_socketxtreme = true,
        .optimize_for_latency = true,
        .use_polling = true,
        .ring_count = 4,
        .buffer_size = BUFFER_SIZE,
        .enable_timestamps = true
    };
    
    if (tcp_socket_init(&server, &options) != TCP_SUCCESS) {
        printf("Failed to initialize server socket\n");
        return;
    }
    
    // Bind to address
    if (tcp_socket_bind(&server, ip, port) != TCP_SUCCESS) {
        printf("Failed to bind server socket\n");
        tcp_socket_close(&server);
        return;
    }
    
    // Listen for connections
    if (tcp_socket_listen(&server, 10) != TCP_SUCCESS) {
        printf("Failed to listen on server socket\n");
        tcp_socket_close(&server);
        return;
    }
    
    printf("TCP server listening on %s:%d\n", ip ? ip : "0.0.0.0", port);
    
    // Start time measurement
    struct timeval start_time, current_time;
    gettimeofday(&start_time, NULL);
    
    // Main server loop
    while (running) {
        // Check test duration
        gettimeofday(&current_time, NULL);
        double elapsed = (current_time.tv_sec - start_time.tv_sec) + 
                        (current_time.tv_usec - start_time.tv_usec) / 1000000.0;
        if (elapsed >= TEST_DURATION) {
            break;
        }
        
        // Accept new client
        tcp_client_t client;
        tcp_result_t result = tcp_socket_accept(&server, &client, 1000); // 1s timeout
        
        if (result == TCP_SUCCESS) {
            char ip_str[INET_ADDRSTRLEN];
            inet_ntop(AF_INET, &client.addr.sin_addr, ip_str, INET_ADDRSTRLEN);
            printf("Client connected from %s:%d\n", ip_str, ntohs(client.addr.sin_port));
            
            // Create thread for client handling
            client_thread_data_t* thread_data = malloc(sizeof(client_thread_data_t));
            if (!thread_data) {
                printf("Failed to allocate memory for thread data\n");
                tcp_socket_close_client(&client);
                continue;
            }
            
            thread_data->client = client;
            thread_data->running = &running;
            
            pthread_t client_thread;
            if (pthread_create(&client_thread, NULL, client_handler_thread, thread_data) != 0) {
                printf("Failed to create client thread\n");
                tcp_socket_close_client(&client);
                free(thread_data);
                continue;
            }
            
            // Detach thread - we don't need to join it
            pthread_detach(client_thread);
        } else if (result == TCP_ERROR_TIMEOUT) {
            // Timeout - continue
        } else {
            printf("Error accepting client: %d\n", result);
            break;
        }
    }
    
    // Get final time
    gettimeofday(&current_time, NULL);
    double elapsed = (current_time.tv_sec - start_time.tv_sec) + 
                    (current_time.tv_usec - start_time.tv_usec) / 1000000.0;
    
    // Close server socket
    tcp_socket_close(&server);
    
    // Print results
    printf("\n====== Test Results ======\n");
    printf("Total bytes received: %lu\n", bytes_received);
    printf("Throughput: %.2f Mbps\n", 8.0 * bytes_received / elapsed / 1000000.0);
    
    // Give client threads some time to finish
    sleep(1);
}

// Client function
void run_client(const char* ip, uint16_t port) {
    // Initialize TCP socket
    tcp_socket_t client;
    tcp_vma_options_t options = {
        .use_socketxtreme = true,
        .optimize_for_latency = true,
        .use_polling = true,
        .ring_count = 4,
        .buffer_size = BUFFER_SIZE,
        .enable_timestamps = true
    };
    
    if (tcp_socket_init(&client, &options) != TCP_SUCCESS) {
        printf("Failed to initialize client socket\n");
        return;
    }
    
    // Connect to server
    printf("Connecting to %s:%d...\n", ip, port);
    if (tcp_socket_connect(&client, ip, port, 5000) != TCP_SUCCESS) {
        printf("Failed to connect to server\n");
        tcp_socket_close(&client);
        return;
    }
    
    printf("Connected to server\n");
    
    // Prepare test data
    char buffer[BUFFER_SIZE];
    memset(buffer, 'A', BUFFER_SIZE);
    
    // Start time measurement
    struct timeval start_time, current_time;
    gettimeofday(&start_time, NULL);
    
    // Main client loop - send data
    while (running) {
        // Check test duration
        gettimeofday(&current_time, NULL);
        double elapsed = (current_time.tv_sec - start_time.tv_sec) + 
                        (current_time.tv_usec - start_time.tv_usec) / 1000000.0;
        if (elapsed >= TEST_DURATION) {
            break;
        }
        
        // Check connection status
        if (!tcp_socket_is_connected(&client)) {
            printf("Connection lost, trying to reconnect...\n");
            if (tcp_socket_reconnect(&client, 1000) != TCP_SUCCESS) {
                printf("Failed to reconnect\n");
                break;
            }
            printf("Reconnected\n");
        }
        
        // Send data
        size_t bytes_sent_now = 0;
        tcp_result_t result = tcp_socket_send(&client, buffer, BUFFER_SIZE, &bytes_sent_now);
        
        if (result == TCP_SUCCESS) {
            bytes_sent += bytes_sent_now;
            
            // Print status every 1MB
            if ((bytes_sent / (1024 * 1024)) != ((bytes_sent - bytes_sent_now) / (1024 * 1024))) {
                printf("Sent %lu MB\n", bytes_sent / (1024 * 1024));
            }
        } else if (result == TCP_ERROR_WOULD_BLOCK) {
            // Would block - small delay
            usleep(10); // 10 microseconds
        } else {
            printf("Error sending data: %d\n", result);
            break;
        }
    }
    
    // Get final time
    gettimeofday(&current_time, NULL);
    double elapsed = (current_time.tv_sec - start_time.tv_sec) + 
                    (current_time.tv_usec - start_time.tv_usec) / 1000000.0;
    
    // Close client socket
    tcp_socket_close(&client);
    
    // Print results
    printf("\n====== Test Results ======\n");
    printf("Total bytes sent: %lu\n", bytes_sent);
    printf("Throughput: %.2f Mbps\n", 8.0 * bytes_sent / elapsed / 1000000.0);
}

int main(int argc, char* argv[]) {
    // Parse command line arguments
    if (argc < 2) {
        printf("Usage: %s [server|client] [ip] [port]\n", argv[0]);
        printf("  Default: 127.0.0.1:%d\n", DEFAULT_PORT);
        return 1;
    }
    
    const char* mode = argv[1];
    const char* ip = (argc > 2) ? argv[2] : "127.0.0.1";
    uint16_t port = (argc > 3) ? (uint16_t)atoi(argv[3]) : DEFAULT_PORT;
    
    // Set up signal handler for Ctrl+C
    signal(SIGINT, signal_handler);
    
    if (strcmp(mode, "server") == 0) {
        printf("Starting TCP server mode on %s:%d\n", ip, port);
        run_server(ip, port);
    } else if (strcmp(mode, "client") == 0) {
        printf("Starting TCP client mode to %s:%d\n", ip, port);
        run_client(ip, port);
    } else {
        printf("Unknown mode: %s\n", mode);
        printf("Use 'server' or 'client'\n");
        return 1;
    }
    
    return 0;
}