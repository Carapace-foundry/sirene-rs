# Data Ingestion

`sirene-rs` supports direct streaming ingestion of official French public datasets published on [data.gouv.fr](https://data.gouv.fr).

---

## 📂 Source Datasets

### 1. Base SIRENE (INSEE)
- **`StockUniteLegale_utf8.zip`**: All legal units in France (~25 million records).
  - Primary keys: `siren`, `denominationUniteLegale`, `nomUniteLegale`, `activitePrincipaleUniteLegale`, `etatAdministratifUniteLegale`.
- **`StockEtablissement_utf8.zip`**: All individual establishments (~35 million records).
  - Primary keys: `siret`, `nic`, `codeCommuneEtablissement`, `codePostalEtablissement`.

### 2. Registre National des Entreprises - RNE (INPI)
- Corporate representatives and legal directors (`nom`, `prenoms`, `denomination`).

---

## ⚡ Streaming Parser

The ingestion pipeline (`sirene-cli::ingest`) reads compressed `.zip` and `.csv.gz` streams on the fly without decompressing them to disk first:

```bash
sirene index \
  --unite-legale StockUniteLegale_utf8.zip \
  --etablissement StockEtablissement_utf8.zip \
  --dirigeants Dirigeants.csv \
  --output sirene.idx
```

### Ingestion Filters

- `--active-only`: Only index currently active enterprises (`etatAdministratif = 'A'`), dramatically reducing index size.
- `--department <DEP>`: Filter index creation to a specific department (e.g. `75`, `69`, `33`, `2A`).
- `--limit <N>`: Maximum number of records to ingest (ideal for testing).
