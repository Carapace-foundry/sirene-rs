# Frequently Asked Questions (FAQ)

### How is `sirene-rs` different from Elasticsearch or OpenSearch?
Elasticsearch requires a dedicated JVM cluster, complex shard topologies, and substantial RAM buffers. `sirene-rs` compiles into a single native binary that uses deterministic FSTs and Roaring Bitmaps with memory-mapped files (`memmap2`). It requires zero background daemons, consumes minimal RAM, and cold starts in milliseconds.

### What is the La Poste Luhn exception?
Standard SIRET numbers must satisfy the modulus-10 Luhn formula across all 14 digits. However, historical establishments belonging to *La Poste* (starting with `356000000`) use an alternate modulus sum calculation rule defined by INSEE. `sirene-rs` natively implements both the standard Luhn check and the La Poste derogation.

### Can `sirene-rs` run entirely in the browser?
Yes! The `sirene-wasm` crate exposes the full engine via `wasm-bindgen`. You can load pre-built `.idx` files (e.g. for a specific region or sector) directly into memory in the browser or in a Progressive Web App (PWA).

### What official data files are needed to build a complete index?
You need:
1. `StockUniteLegale_utf8.zip` (legal companies from INSEE).
2. (Optional) `StockEtablissement_utf8.zip` (establishments, addresses, postal codes).
3. (Optional) RNE Dirigeants extract from INPI.

All are freely accessible under open licenses on [data.gouv.fr](https://data.gouv.fr).
