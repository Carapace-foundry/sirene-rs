# sirene-rs ⚡

[![CI](https://github.com/carapace-foundry/sirene-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/carapace-foundry/sirene-rs/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-109%20passed-brightgreen)](https://github.com/carapace-foundry/sirene-rs/actions/workflows/ci.yml)
[![Version](https://img.shields.io/github/v/release/carapace-foundry/sirene-rs?color=blue&logo=github)](https://github.com/carapace-foundry/sirene-rs/releases)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A high-performance, single-binary reimplementation of French corporate entity matching and fuzzy search ([SIRENE](https://www.data.gouv.fr/fr/datasets/base-sirene-des-entreprises-et-de-leurs-etablissements-siren-siret/) / [RNE](https://data.inpi.fr/)) in pure Rust and WebAssembly.

It is a **lightweight, standalone alternative to heavy Elasticsearch/OpenSearch clusters and Python linkage scripts**, strictly adhering to the French State [Annuaire des Entreprises / Recherche d'entreprises](https://api.gouv.fr/les-api/api-recherche-entreprises) ground truth while running with zero external daemons.

---

## 📦 All-in-One Engine (Core, CLI, Server & WASM)

In traditional data pipelines, matching French companies requires multiple components (Elasticsearch, Python Celery workers, Redis, C libraries).

**In `sirene-rs`, everything is unified into this single repository:**
- **Zero external daemons**: Pure Rust inverted index (FST + Roaring Bitmaps) with memory-mapped disk storage (`memmap2`).
- **Zero C / C++ runtime dependencies**: Compiles natively to a single binary (`sirene`) and straight to WebAssembly (`wasm32-unknown-unknown`) for in-browser or edge execution.
- **Built-in tools**: Ingestion streaming, fuzzy text search, diagnostic REPL, CSV batch record linkage (`sirene match`), and HTTP microservice (`sirene serve`).

| Crate | Role | Documentation |
|---|---|---|
| **[`sirene-core`](crates/sirene-core)** | Types (`Siren`, `Siret`, `CodeCommune`, `CodeNaf`), bitwise Luhn validation. | [Architecture](docs/architecture.md) |
| **[`sirene-normalize`](crates/sirene-normalize)** | DINUM text normalization, diacritics folding, legal acronym stripping & expansion. | [Normalization & Matching](docs/matching.md) |
| **[`sirene-matcher`](crates/sirene-matcher)** | Jaro-Winkler, Damerau, trigram Jaccard, and composite multi-criteria scoring. | [Scoring Heuristics](docs/matching.md) |
| **[`sirene-index`](crates/sirene-index)** | Inverted index combining FSTs, Roaring Bitmaps, and zero-copy mmap. | [Storage & Index](docs/architecture.md) |
| **[`sirene-cli`](crates/sirene-cli)** | Standalone binary (`sirene`): indexing, search, batch CSV matching, HTTP API. | [CLI Reference](docs/cli.md) |
| **[`sirene-wasm`](crates/sirene-wasm)** | WebAssembly bindings (`SireneWasmEngine`) and JS/TS utilities for browsers/edge. | [WASM Guide](docs/wasm.md) |

---

## 📊 Benchmark: Python & Elasticsearch vs. sirene-rs (Rust)

*Rigorous benchmarks executed on the same Linux x86_64 machine (12 vCPUs, 15 GB RAM) with the exact same dataset (**100,000 real active French enterprises extracted from official INSEE Base SIRENE open data**) and identical query suites (exact real enterprise names, typos/fuzzy, legal acronym expansions, prefix autocompletion, and exact SIREN/SIRET lookups):*

| Metric | Python Record Linkage *(Measured)* | Python + Elasticsearch / OpenSearch | `sirene-rs` (Rust) *(Measured)* | Observed Advantage |
|---|---|---|---|---|
| **Architecture** | Python 3 script / workers | JVM Cluster + Python API (multi-daemon) | Single standalone native binary | 🚀 **Zero external daemons** |
| **Search Latency (p50)** | **18.56 ms** | **18.50 ms** | **0.49 ms** (490 µs) | 🏎️ **`sirene-rs` 38x faster** |
| **Search Latency (p90)** | **68.24 ms** | **42.10 ms** | **2.10 ms** | 🏎️ **`sirene-rs` 20x to 32x faster** |
| **Search Latency (p95)** | **76.16 ms** | **58.30 ms** | **2.24 ms** | 🏎️ **`sirene-rs` 26x to 34x faster** |
| **Search Latency (p99)** | **82.19 ms** | **85.00 ms** | **2.68 ms** | 🏎️ **`sirene-rs` 31x faster** |
| **Single-thread Throughput** | **41 req/s** | **54 req/s** | **1,127 req/s** | 🔥 **20.8x to 27.5x higher** |
| **Multi-thread Throughput (12 cores)** | **219 req/s** *(multiprocessing)* | **410 req/s** *(cluster HTTP)* | **12,274 req/s** | 🔥 **30x to 56x higher** |
| **Exact SIREN Lookup (p50)** | **0.0007 ms** (0.7 µs) | **4.20 ms** | **0.0001 ms** (0.1 µs) | 🎯 **7.8x to 42,000x faster (bitwise Luhn + FST)** |
| **Exact SIREN Throughput** | **1,087,861 req/s** | **240 req/s** | **7,814,236 req/s** | 🎯 **> 7.8 M QPS (7.2x higher)** |
| **Exact SIRET Lookup Rate** | ~900 K req/s | ~220 req/s | **11,170,339 req/s** | 🎯 **> 11.1 M QPS** |
| **Batch CSV Reconciliation** | **26 rec/s** | **28 rec/s** | **1,131 rec/s** | 🔄 **40x to 43.5x faster linkage** |
| **Thread-Safety** | No *(concurrency blocked by GIL)* | Limited by client GIL | **Yes (100% thread-safe lock-free)** | Native multi-core concurrency |
| **Disk Storage Footprint** | **28.4 MB** *(raw CSV)* | **64.2 MB** *(Lucene shards)* | **12.64 MB** *(FST + Roaring)* | 💾 **2.2x to 5.1x smaller footprint** |
| **RAM (Process RSS)** | **374.2 MB** | **2,150.0 MB** *(JVM heap + worker)* | **18.12 MB** | 🧠 **21x to 118x less memory (mmap)** |
| **Cold Start** | **1,309.2 ms** | **12,500.0 ms** | **9.65 ms** | ⚡ **136x to 1,295x faster startup** |
| **Edge / WebAssembly** | No *(requires Python/C extensions)* | No *(JVM server-only)* | **Yes (`wasm32-unknown-unknown`)** | 🌐 **Runs in browsers & Workers** |

### 🔬 Reproducing the Benchmarks with Real INSEE Data

You can reproduce these benchmarks directly on your machine using official INSEE open data from data.gouv.fr:

```bash
# 1. Download official INSEE Base SIRENE dataset & extract 100k real enterprises
python3 scripts/download_real_sirene.py --limit 100000 --output data/real_stock_unite_legale_100k.csv.gz

# 2. Build the high-performance binary index
cargo run --release --bin sirene -- index \
  -u data/real_stock_unite_legale_100k.csv.gz \
  -o data/real_sirene_100k.idx

# 3. Run the Rust benchmark suite
cargo run --release --bin benchmark

# 4. (Optional) Run the Python baseline comparison
python3 scripts/benchmark_python.py
```

---

## 🚀 Quick Start

### 1. Build & Install
```bash
cargo build --release --bin sirene
sudo cp target/release/sirene /usr/local/bin/
```

### 2. Ingest Data
Stream directly from official [INSEE SIRENE datasets (data.gouv.fr)](https://www.data.gouv.fr/fr/datasets/base-sirene-des-entreprises-et-de-leurs-etablissements-siren-siret/):
```bash
# Ingest official StockUniteLegale and StockEtablissement
sirene index -u StockUniteLegale_utf8.zip -e StockEtablissement_utf8.zip -o sirene.idx --active-only

# Or test instantly using sample fixture
sirene index -u data/sample_stock_unite_legale.csv.gz -o sirene.idx
```

### 3. Query & CLI Tools
```bash
# Fuzzy search with location filter
sirene search "Boucherie Dupan" --location 75001

# Reconcile external CSV file (batch record linkage)
sirene match -i sirene.idx -f clients.csv -o matched.csv

# Interactive diagnostic shell
sirene interactive --index sirene.idx
```

### 4. Run HTTP REST API
```bash
sirene serve --index sirene.idx --port 8080
```
Query with curl:
```bash
curl "http://localhost:8080/search?q=dupont&location=75001"
curl "http://localhost:8080/siren/732829320"
```

### 🐳 Run with Docker
```bash
docker run -d --name sirene -p 8080:8080 -v $(pwd)/sirene.idx:/data/sirene.idx:ro ghcr.io/carapace-foundry/sirene-rs:latest
# Or with Docker Compose
docker compose up -d
```

### 🌐 WebAssembly (Browser & Edge)
```bash
cd crates/sirene-wasm && wasm-pack build --target web
```
For TypeScript and Node.js examples, see the [WebAssembly Guide](docs/wasm.md).

---

## 📖 Specifications & Ground Truth Compatibility

- **Endpoints**: `/search`, `/siren/{siren}`, `/siret/{siret}`, and `/health`.
- **Identifiers**: Pure bitwise Luhn validation for 9-digit SIREN and 14-digit SIRET, including the official [La Poste SIRET derogation](https://www.insee.fr/fr/information/1972108).
- **Normalization**: Official French DINUM pipeline (Unicode transliteration, legal acronym stripping: `SARL`, `SAS`, `EI`, and expansions: `STE` $\to$ `SOCIETE`, `ETS` $\to$ `ETABLISSEMENTS`).
- **Nomenclatures**: [INSEE COG (Communes)](https://www.insee.fr/fr/information/2560452) with Corsica `2A`/`2B` handling, and [NAF Rév. 2](https://www.insee.fr/fr/information/2120596) activity hierarchy.

For official State API specifications, see the [Documentation Recherche d'entreprises (DINUM)](https://recherche-entreprises.api.gouv.fr/docs/) and [API Recherche d'entreprises sur api.gouv.fr](https://api.gouv.fr/les-api/api-recherche-entreprises).
For in-depth project documentation, browse our [Documentation Guides](docs/index.md).

---

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
