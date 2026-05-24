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

```bash
# clone repository
git clone [https://github.com/gvedant666/kvstore.git](https://github.com/gvedant666/kvstore.git)
cd kvstore

# Build and run the server in highly-optimized release mode
cargo run --release --bin kvstore
