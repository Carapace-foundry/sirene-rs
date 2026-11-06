# CLI Command Reference

`sirene` provides a comprehensive command-line interface for indexing, querying, serving, and reconciling French company datasets.

---

## Commands Summary

| Command | Description |
|---|---|
| `sirene index` | Ingest raw INSEE CSV / ZIP files and compile a compact `.idx` file. |
| `sirene search` | Perform fuzzy corporate entity search via CLI. |
| `sirene serve` | Run the high-performance HTTP microservice. |
| `sirene match` | Batch match external CSV datasets against SIRENE. |
| `sirene interactive` | Interactive REPL shell for debugging candidate scoring. |
| `sirene inspect` | Inspect binary index segments, document counts, and statistics. |

---

## Command Syntax & Options

### `sirene index`
```bash
sirene index [OPTIONS] --unite-legale <FILE>
```
- `-u, --unite-legale <PATH>`: Path to `StockUniteLegale` CSV, ZIP, or GZ.
- `-e, --etablissement <PATH>`: Optional path to `StockEtablissement` file.
- `-d, --dirigeants <PATH>`: Optional path to RNE Dirigeants CSV.
- `-o, --output <PATH>`: Output binary index file (default: `sirene.idx`).
- `--active-only`: Index only active businesses.
- `--department <DEP>`: Restrict index to a single department (e.g. `75`, `2A`).
- `-l, --limit <N>`: Maximum records to ingest.

---

### `sirene search`
```bash
sirene search [OPTIONS] <QUERY>
```
- `-i, --index <PATH>`: Path to binary index file (default: `sirene.idx`).
- `-l, --location <LOC>`: Department, postal code, or COG commune code.
- `--naf <CODE>`: NAF activity code (e.g. `62.01Z`).
- `--dirigeant <NAME>`: Corporate executive name.
- `-k, --top-k <N>`: Maximum candidates to display (default: 10).
- `--json`: Output results in JSON format.

---

### `sirene serve`
```bash
sirene serve [OPTIONS]
```
- `-i, --index <PATH>`: Path to index file (default: `sirene.idx`).
- `-p, --port <PORT>`: HTTP listening port (default: 8080).
- `-b, --host <HOST>`: Binding address (default: `0.0.0.0`).
- `-w, --workers <N>`: Number of worker threads.

---

### `sirene match`
```bash
sirene match [OPTIONS] --input <CSV_FILE>
```
- `-i, --index <PATH>`: Path to index file (default: `sirene.idx`).
- `-f, --input <PATH>`: External CSV file to match.
- `--col-nom <NAME>`: Column name for entity name (default: `nom`).
- `--col-location <NAME>`: Optional column for postal code or department.
- `--col-naf <NAME>`: Optional column for NAF code.
- `-o, --output <PATH>`: Destination CSV file (default: `matched.csv`).
- `--threshold <SCORE>`: Minimum match acceptance score between 0.0 and 1.0 (default: 0.50).

---

### `sirene interactive`
```bash
sirene interactive --index sirene.idx
```
Launches a diagnostic REPL allowing queries and real-time score breakdowns.

---

### `sirene inspect`
```bash
sirene inspect --index sirene.idx
```
Prints index header version, document counts, token lexicon size, and segment offsets.

---

### `benchmark`
```bash
cargo run --release --bin benchmark
```
Runs the automated benchmark suite measuring cold startup time, memory (RSS), single-threaded latency distribution (p50/p90/p95/p99), multi-threaded throughput (12 cores), exact SIREN/SIRET resolution, and batch CSV reconciliation.
