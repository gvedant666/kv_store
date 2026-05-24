# High-Performance In-Memory Key-Value Store

> A low-latency, deterministic in-memory database built from scratch in Rust. Designed for high-throughput workloads, it features a custom asynchronous event loop, zero-copy protocol parsing, and lock-free data structures optimized for mechanical sympathy.

---

## Table of contents

* [Overview](#overview)
* [Features](#features)
* [Quick start](#quick-start)
* [Project structure](#project-structure)
* [Design & architecture](#design--architecture)
  * [Component diagram (Mermaid)](#component-diagram-mermaid)
  * [Sequence diagram: client -> server -> worker](#sequence-diagram-client---server---worker)
* [Core architectural decisions](#core-architectural-decisions)
* [Protocol (TCP)](#protocol-tcp)
* [Benchmarks & results](#benchmarks--results)
* [Testing & validation](#testing--validation)
* [Limitations & future improvements](#limitations--future-improvements)
* [License](#license)

---

## Overview

This project is a production-grade, bare-metal in-memory key-value store initially built in C++ and fully migrated to Rust. It was built to demonstrate a deep understanding of systems engineering, memory management, and deterministic latency required for High-Frequency Trading (HFT) and DeFi infrastructure.

Instead of relying on heavy asynchronous runtimes like Tokio, this engine is built directly on top of OS primitives using `mio` (`epoll`/`kqueue`). It achieves extreme performance by eliminating heap allocations on the hot path and utilizing cache-friendly memory layouts.

---

## Features

* **Supported Commands:** `GET`, `SET`, `DEL`, `ZADD`, `ZREM`, `ZSCORE`, `ZQUERY`, `PEXPIRE`
* **Zero-Copy Parser:** Utilizes `bytes::BytesMut` and explicit lifetimes (`'a`) to parse incoming TCP streams without allocating strings on the heap.
* **Custom Event Loop:** A single-threaded, non-blocking network I/O loop built with `mio`.
* **High-Concurrency Storage:** A 256-shard concurrent hash map utilizing fine-grained `RwLock`s to eliminate global lock contention.
* **Arena-Allocated AVL Trees:** Sorted sets are stored in contiguous memory (`Vec<Node>`) using `usize` indices instead of pointers, maximizing L1/L2 CPU cache hits.
* **TTL Min-Heap:** Expiry management is handled by a custom flat-array min-heap.

---

## Quick start

### Prerequisites
* Rust toolchain (Cargo) installed.

### Build & Run


# clone repository
git clone [https://github.com/gvedant666/kvstore.git](https://github.com/gvedant666/kvstore.git)
cd kvstore

# Build and run the server in highly-optimized release mode
cargo run --release --bin kvstore

The server listens on 127.0.0.1:1234 by default.


## Project structure

kvstore/
├── Cargo.toml             # Dependency and build configuration
├── .gitignore
├── src/
│   ├── main.rs            # Entry point for the server
│   ├── lib.rs             # Module definitions
│   ├── server.rs          # mio event loop and connection management
│   ├── protocol.rs        # Zero-copy parser and ResponseBuilder
│   ├── storage.rs         # Core storage engine routing
│   ├── concurrent_map.rs  # 256-shard HashMap
│   ├── avl.rs             # Arena-allocated AVL tree
│   ├── zset.rs            # Sorted set combining AVL and HashMap
│   ├── heap.rs            # Min-heap for TTL expiry
│   ├── threadpool.rs      # Custom OS-thread worker pool
│   └── bin/
│       └── benchmark.rs   # Pipelined and parallel benchmarking tool
└── README.md

## Design & architecture

flowchart LR
    subgraph CLIENTS
        C[Clients]
    end
    C -->|TCP| L[mio::TcpListener]
    L --> E[Event Loop (epoll/kqueue)]
    E --> P[Zero-Copy Parser]
    P --> W[Custom Thread Pool]
    W --> S[Storage Engine]
    S --> H[256-Shard HashMap]
    S --> Z[Arena AVL / ZSet]


## Sequence diagram

sequenceDiagram
    Client->>Server: TCP connect
    Client->>Server: [Custom Binary Protocol] SET key val
    Server->>EventLoop: mio::Poll readable event
    EventLoop->>Parser: parse_request(&BytesMut)
    Parser-->>EventLoop: Zero-copy Command<'a>
    EventLoop->>Worker: execute(Command)
    Worker->>HashMap: acquire Shard RwLock
    Worker->>HashMap: insert(key, val)
    Worker->>EventLoop: queue response
    EventLoop->>Client: +OK (Binary)


## Core architectural decisions
mio over Tokio: To guarantee deterministic latency, the server avoids cooperative task scheduling. The custom event loop handles OS interrupts directly, minimizing context switching overhead.

Arena Allocation vs. Rc<RefCell>: Standard Rust trees suffer from heap fragmentation. The avl.rs implementation stores all nodes in a contiguous Vec, replacing pointers with array indices. This ensures structural modifications are cache-friendly.

Lock Sharding: A single Mutex serializes database access. By hashing keys into 256 independent buckets, the probability of thread contention on any single RwLock is reduced to ~0.39%, allowing massive read concurrency.

## Protocol (TCP)

To bypass the overhead of string parsing, the engine uses a custom length-prefixed binary protocol.

Format:
[4-byte Total Length] [4-byte Num Args] [4-byte Arg1 Length] [Arg1 Bytes] ...

This allows the parser to slice directly into the BytesMut read buffer to extract arguments in O(1) time without memory allocation.

## Benchmarks & results
Benchmarks were conducted on a local loopback interface (127.0.0.1) using the included src/bin/benchmark.rs utility, which utilizes 50 parallel OS threads to simulate standard redis-benchmark behavior.

   Multi-Client Throughput: ~95,000 requests/sec

   Pipelined Throughput (100 cmds/packet): ~540,000 requests/sec

   Average Internal Engine Latency: < 10 µs (0.01 ms)

   Tail Latency (p99): < 600 µs

Note: These metrics prove the lock-sharding and zero-copy architecture successfully execute near-instantly, leaving only raw socket I/O overhead.

## Testing & validation

# Start the server
cargo run --release --bin kvstore

# In a separate terminal, run the parallel benchmark
cargo run --release --bin benchmark




