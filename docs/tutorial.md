# Quickstart Tutorial

This step-by-step tutorial walks through indexing sample data, querying companies via CLI, running the HTTP REST server, and reconciling records in batch.

---

## 1. Indexing Sample Data

The repository includes a sample dataset in `data/sample_stock_unite_legale.csv.gz`.

Run the indexer:
```bash
sirene index \
  --unite-legale data/sample_stock_unite_legale.csv.gz \
  --output sirene.idx
```

Inspect the generated index:
```bash
sirene inspect --index sirene.idx
```

---

## 2. Searching Companies

### Simple Query
```bash
sirene search "Boucherie Dupont" --index sirene.idx
```

### Tolerating Typos & Abbreviations
```bash
sirene search "Boucherie Dupan" --index sirene.idx
sirene search "SNCF" --index sirene.idx
```

### Applying Geographic & NAF Filters
```bash
# Filter by Paris (75001) and IT activities (NAF 62.01Z)
sirene search "Tech Solutions" --location 75001 --naf 62.01Z --index sirene.idx
```

---

## 3. Running the REST HTTP Server

Start the microservice:
```bash
sirene serve --index sirene.idx --host 127.0.0.1 --port 8080
```

Query endpoints:
```bash
# Check status
curl http://127.0.0.1:8080/health

# Search
curl "http://127.0.0.1:8080/search?q=dupont&limit=5"

# Look up by SIREN
curl http://127.0.0.1:8080/siren/732829320
```

---

## 4. Batch Reconciliation

Given an external CSV file (`clients.csv`):
```csv
id,nom,code_postal
1,Boucherie Dupan,75001
2,Boulangerie Martin SARL,33000
```

Reconcile against SIRENE:
```bash
sirene match \
  --index sirene.idx \
  --input clients.csv \
  --col-nom "nom" \
  --col-location "code_postal" \
  --output matched.csv
```
