/**
 * udp_test.c - Example of using UDP Socket structure
 * 
 * Compile: gcc -o udp_test udp_test.c udp_socket.c vma_common.c -pthread
 * Run: LD_PRELOAD=/usr/lib64/libvma.so.9.8.51 ./udp_test
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <stdint.h>
#include <stdbool.h>
#include <signal.h>
#include "udp_socket.h"
#include "vma_common.h"

#define BUFFER_SIZE 8192
#define TEST_DURATION 10  // Test duration (seconds)

volatile int running = 1;
uint64_t packets_sent = 0;
uint64_t packets_received = 0;

// Signal handler for termination
void signal_handler(int sig) {
    running = 0;
    printf("Received termination signal, ending test...\n");
}

// Sender thread function
void* sender_thread(void* arg) {
    // Initialize UDP socket
    udp_socket_t sender;
    vma_options_t options = {
        .use_socketxtreme = true,
        .optimize_for_latency = true,
        .use_polling = true,
        .ring_count = 4,
        .buffer_size = BUFFER_SIZE,
        .enable_timestamps = true
    };
    
    if (udp_socket_init(&sender, &options) != UDP_SUCCESS) {
        printf("Failed to initialize sender socket\n");
        return NULL;
    }
    
    // Connect to 127.0.0.1:5001
    if (udp_socket_connect(&sender, "127.0.0.1", 5001) != UDP_SUCCESS) {
        printf("Failed to connect sender socket\n");
        udp_socket_close(&sender);
        return NULL;
    }
    
    // Prepare test data
    char buffer[BUFFER_SIZE];
    memset(buffer, 'A', BUFFER_SIZE);
    
    printf("Sender thread started\n");
    
    // Sending loop
    while (running) {
        size_t bytes_sent;
        if (udp_socket_send(&sender, buffer, BUFFER_SIZE, &bytes_sent) == UDP_SUCCESS) {
            packets_sent++;
        }
        
        // Small delay to prevent too fast sending rate
        usleep(10);  // 10 microseconds delay
    }
    
    // Close socket
    udp_socket_close(&sender);
    
    printf("Sender thread ended: %lu packets sent\n", packets_sent);
    return NULL;
}

// Receiver thread function
void* receiver_thread(void* arg) {
    // Initialize UDP socket
    udp_socket_t receiver;
    vma_options_t options = {
        .use_socketxtreme = true,
        .optimize_for_latency = true,
        .use_polling = true,
        .ring_count = 4,
        .buffer_size = BUFFER_SIZE,
        .enable_timestamps = true
    };
    
    if (udp_socket_init(&receiver, &options) != UDP_SUCCESS) {
        printf("Failed to initialize receiver socket\n");
        return NULL;
    }
    
    // Bind to port 5001
    if (udp_socket_bind(&receiver, NULL, 5001) != UDP_SUCCESS) {
        printf("Failed to bind receiver socket\n");
        udp_socket_close(&receiver);
        return NULL;
    }
    
    // Receive buffer
    char buffer[BUFFER_SIZE];
    udp_packet_t packet = {0};
    
    printf("Receiver thread started\n");
    
    // Receiving loop
    while (running) {
        // Receive packet with 100ms timeout
        if (udp_socket_recvfrom(&receiver, &packet, buffer, BUFFER_SIZE, 100) == UDP_SUCCESS) {
            packets_received++;
        }
    }
    
    // Close socket
    udp_socket_close(&receiver);
    
    printf("Receiver thread ended: %lu packets received\n", packets_received);
    return NULL;
}

int main() {
    // Set up signal handler for interrupt
    signal(SIGINT, signal_handler);
    
    // Create threads
    pthread_t sender, receiver;
    
    // Create receiver thread
    if (pthread_create(&receiver, NULL, receiver_thread, NULL) != 0) {
        printf("Failed to create receiver thread\n");
        return EXIT_FAILURE;
    }
    
    // Wait a moment to allow receiver thread to complete binding
    usleep(100000);  // 0.1 seconds
    
    // Create sender thread
    if (pthread_create(&sender, NULL, sender_thread, NULL) != 0) {
        printf("Failed to create sender thread\n");
        running = 0;
        pthread_join(receiver, NULL);
        return EXIT_FAILURE;
    }
    
    printf("Test running... will run for %d seconds.\n", TEST_DURATION);
    sleep(TEST_DURATION);
    running = 0;
    
    // Wait for threads to finish
    pthread_join(sender, NULL);
    pthread_join(receiver, NULL);
    
    // Print results
    printf("\n====== Test Results ======\n");
    printf("Total packets sent: %lu\n", packets_sent);
    printf("Total packets received: %lu\n", packets_received);
    printf("Packet loss rate: %.2f%%\n", 100.0 * (packets_sent - packets_received) / packets_sent);
    printf("Throughput: %.2f Mbps\n", 8.0 * BUFFER_SIZE * packets_received / TEST_DURATION / 1000000);
    
    return EXIT_SUCCESS;
}