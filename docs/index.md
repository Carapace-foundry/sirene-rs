# Welcome to sirene-rs ⚡

*Ultra-fast, deterministic SIRENE / RNE fuzzy matching and entity resolution in pure Rust and WebAssembly.*

[sirene-rs](https://github.com/carapace-foundry/sirene-rs) is a high-performance, single-binary reimplementation of French corporate entity matching and indexing. It replaces heavy Elasticsearch / OpenSearch clusters and memory-intensive Python linkage scripts with an embedded, zero-dependency engine.

---

## 🚀 Key Features

- **Standalone Native Binary**: Compiles into a single binary (`sirene`). No external runtime or daemons.
- **Zero C / C++ Dependencies**: 100% pure Rust compiled natively and directly to `wasm32-unknown-unknown`.
- **FST & Roaring Bitmaps Index**: Compact binary inverted index queried via memory-mapped disk access (`memmap2`).
- **Ground Truth Compliance**:
  - Bitwise Luhn algorithm for 9-digit SIREN and 14-digit SIRET (with La Poste exception).
  - DINUM official text normalization (Unicode diacritic removal, ligatures folding, French stop words, and legal acronym removal).
  - COG commune codes (with Corsica 2A/2B handling) and NAF Rev. 2 activity codes.
- **Multi-Criteria Scoring Engine**: Combines Jaro-Winkler distance, trigram Jaccard similarity, geographic proximity, NAF hierarchy, RNE dirigeants, and active enterprise status.
- **Built-in HTTP Microservice**: Single command `sirene serve` exposes REST endpoints (`/search`, `/siren/{siren}`, `/siret/{siret}`, `/health`).
- **Batch CSV Reconciliation**: Directly reconcile millions of external corporate records with `sirene match`.
- **WebAssembly Engine**: Run the complete search engine client-side in browsers, Node.js, Deno, Bun, or Cloudflare Workers.

---

## ⚡ Performance at a Glance (100,000 Real INSEE Enterprises)

*Measured on Linux x86_64 (12 vCPUs, 15 GB RAM) using official INSEE Base SIRENE open data:*

| Metric | Python Record Linkage *(Measured)* | Python + Elasticsearch | `sirene-rs` (Rust) *(Measured)* | Improvement |
|---|---|---|---|---|
| **Architecture** | Python 3 script / workers | JVM Cluster + Python API | Single standalone binary | 🚀 **Zero external daemons** |
| **Search Latency (p50)** | **18.56 ms** | **18.50 ms** | **0.49 ms** (490 µs) | 🏎️ **38x faster** |
| **Search Latency (p90)** | **68.24 ms** | **42.10 ms** | **2.10 ms** | 🏎️ **20x to 32x faster** |
| **Search Latency (p95)** | **76.16 ms** | **58.30 ms** | **2.24 ms** | 🏎️ **26x to 34x faster** |
| **Search Latency (p99)** | **82.19 ms** | **85.00 ms** | **2.68 ms** | 🏎️ **31x faster** |
| **Throughput (12 cores)** | **219 req/s** | **410 req/s** | **12,274 req/s** | 🔥 **30x to 56x higher** |
| **Exact SIREN Lookup (p50)** | **0.0007 ms** (0.7 µs) | **4.20 ms** | **0.0001 ms** (0.1 µs) | 🎯 **7.8x faster** |
| **Exact SIREN QPS** | **1,087,861 req/s** | **240 req/s** | **7,814,236 req/s** | 🎯 **> 7.8 M QPS** |
| **Exact SIRET QPS** | ~900 K req/s | ~220 req/s | **11,170,339 req/s** | 🎯 **> 11.1 M QPS** |
| **Batch CSV Linkage** | **26 rec/s** | **28 rec/s** | **1,131 rec/s** | 🔄 **43.5x faster** |
| **Process RAM (RSS)** | **374.2 MB** | **2,150.0 MB** | **18.12 MB** | 🧠 **21x to 118x less RAM** |
| **Cold Startup Time** | **1,309.2 ms** | **12,500.0 ms** | **9.65 ms** | ⚡ **136x faster** |
| **WebAssembly / Edge** | No | No | **Yes (`wasm32-unknown-unknown`)** | 🌐 **Runs in browsers & Workers** |

---

## 📚 Documentation Navigation

- [Installation](install.md) — Prerequisites and build instructions.
- [Quickstart Tutorial](tutorial.md) — Index sample data and run your first searches.
- [Architecture & Storage](architecture.md) — FSTs, Roaring Bitmaps, and zero-copy mmap.
- [Data Ingestion](ingest.md) — Streaming INSEE and INPI files (`StockUniteLegale`, `StockEtablissement`).
- [Matching & Scoring](matching.md) — String distances, token overlaps, and composite scoring formula.
- [HTTP REST API](api.md) — Microservice endpoints, query parameters, and JSON specifications.
- [CLI Reference](cli.md) — Command-line syntax for `index`, `search`, `serve`, `match`, `interactive`, and `inspect`.
- [WebAssembly](wasm.md) — Using `sirene-wasm` in JavaScript and TypeScript applications.
- [FAQ](faq.md) — Frequently asked questions.
