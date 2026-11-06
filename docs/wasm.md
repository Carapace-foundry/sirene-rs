# WebAssembly (WASM) Guide

The `sirene-wasm` crate packages the complete `sirene-rs` engine for client-side JavaScript, TypeScript, Node.js, Deno, Bun, and Cloudflare Workers.

---

## 📦 Building the WASM Package

```bash
cd crates/sirene-wasm
wasm-pack build --target web --release
```

Output directory `pkg/` contains:
- `sirene_wasm.js`: JavaScript bridge loader.
- `sirene_wasm_bg.wasm`: Optimized WebAssembly binary.
- `sirene_wasm.d.ts`: Full TypeScript definitions.

---

## 💻 TypeScript Example

```typescript
import init, {
  SireneWasmEngine,
  validateSiren,
  validateSiret,
  normalizeCompany
} from "./pkg/sirene_wasm.js";

async function main() {
  // Initialize WebAssembly runtime
  await init();

  // 1. Instant Luhn validation (zero allocations)
  const isSirenValid = validateSiren("732829320");
  console.log("Is SIREN valid:", isSirenValid); // true

  const isSiretValid = validateSiret("73282932000018");
  console.log("Is SIRET valid:", isSiretValid); // true

  // 2. DINUM text normalization
  const clean = normalizeCompany("SARL Société des Établissements Dupont");
  console.log("Normalized:", clean); // "societe etablissements dupont"

  // 3. In-Memory Search Engine
  const engine = new SireneWasmEngine();

  // Load binary index file fetched via HTTP
  const response = await fetch("/assets/sirene.idx");
  const bytes = new Uint8Array(await response.arrayBuffer());
  engine.load(bytes);

  // Search
  const matches = engine.search("boucherie dupan", "75001", 5);
  console.log("Top candidates:", matches);
}

main();
```

---

## ⚡ Edge & Serverless Execution

Because `sirene-rs` has **zero native C / system library dependencies**, it can be instantiated inside serverless isolate workers (such as Cloudflare Workers or AWS Lambda@Edge) with memory footprints under 64MB for departmental or sample scopes.
