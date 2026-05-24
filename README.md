# In-Memory Data Store — In-Memory Key-Value Store

> A high-performance in-memory key-value store inspired by Redis, designed for low-latency and high-throughput workloads. Implements a custom TCP protocol, efficient data structures (hash table, AVL, zset), and a thread-pool driven server loop.

---

## Table of contents

* [Overview](#overview)
* [Features](#features)
* [Quick start](#quick-start)
* [Project structure](#project-structure)
* [Design & architecture](#design--architecture)

  * [Component diagram (Mermaid)](#component-diagram-mermaid)
  * [Sequence diagram: client -> server -> worker](#sequence-diagram-client---server---worker)
* [Protocol (TCP)](#protocol-tcp)
* [Core data structures](#core-data-structures)
* [Concurrency & networking model](#concurrency--networking-model)
* [Benchmarks & results](#benchmarks--results)

  * [How I measured — commands & scripts](#how-i-measured---commands--scripts)
* [Performance tuning / recommendations](#performance-tuning--recommendations)
* [Testing & validation](#testing--validation)
* [How you can reproduce](#how-you-can-reproduce)
* [Limitations & future improvements](#limitations--future-improvements)
* [Contributing](#contributing)
* [License](#license)
* [Acknowledgements](#acknowledgements)

---

## Overview

In-Memory Data Store is a compact, educational, and production-hardenable in-memory key-value store implemented. It targets real-time and high-throughput applications by combining:

* A **custom TCP command protocol** (simple, text-based inspired by RESP) for client-server communication.
* **Efficient core containers**: hash table for key-value operations, AVL trees for ordered data, and zset for sorted sets.
* **Non-blocking I/O + thread pool** to handle many concurrent clients with low latency.

This README acts as a combined documentation + technical report that explains architecture, protocols, benchmarks, and design decisions.

---

## Features

* Key-Value Operations: `GET`, `SET`, `DEL`, `EXISTS`, `TTL`
* Sorted set operations: `ZADD`, `ZRANGE`, `ZREM`
* Efficient **hash table** for O(1) average lookup and insert.
* **Thread pool** for concurrency.
* **Custom TCP server** with event loop for multiple clients.
* **AVL tree** for ordered operations.
* Modular design for extensions and persistence.

---



---

## Design & architecture

### Component diagram (Mermaid)

```mermaid
flowchart LR
    subgraph CLIENTS
        C[Clients]
    end
    C -->|TCP| L[Listener]
    L --> E[Event Loop]
    E --> W[Thread Pool]
    W --> H[HashTable]
    W --> Z[ZSet/AVL]
```

### Sequence diagram

```mermaid
sequenceDiagram
    Client->>Server: TCP connect
    Client->>Server: SET mykey 0 5\nhello
    Server->>EventLoop: ready
    EventLoop->>Worker: enqueue SET
    Worker->>HashTable: insert("mykey","hello")
    Worker->>Client: +OK
```

---

## Protocol (TCP)

* **SET key expiry length** followed by value
* **GET key** returns value
* **DEL key** removes key
* **ZADD key score value** adds to zset

### Example

```
set mykey 0 5
hello
get mykey
```

Responses:

```
+OK
+VALUE 5
hello
```

---

## Core data structures

* **Hash table** for main KV store
* **AVL tree** for ordered indexes
* **Heap** for expiry management (TTL)
* **ZSet** for sorted collections

---

## Concurrency & networking model

* **Non-blocking TCP server** (`epoll`-like model)
* **Thread pool** distributes requests
* **Locks** used minimally (per-bucket)
* **Memory managed** by pools to reduce allocations

---

## Benchmarks & results

* **Single-thread throughput**: ~90k requests/sec  
* **Latency (average per request)**: ~535 µs (microseconds)  
* **Latency (99th percentile)**: <600 µs

* Tune Linux TCP parameters (buffers, reuse)


## Limitations & future improvements

* Persistence not yet implemented
* No replication/high-availability yet
* Protocol is simple, not RESP-compatible
* Add security (TLS, auth)

---
## Acknowledgements

* Inspired by Redis
* Uses concepts from epoll-based servers and concurrent data structures
