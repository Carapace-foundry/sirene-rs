# HTTP REST API Reference

`sirene-rs` includes a high-performance HTTP microservice command:

```bash
sirene serve --index sirene.idx --host 0.0.0.0 --port 8080
```

---

## Endpoints

### 1. Healthcheck

- **Path**: `GET /health`
- **Response**: `200 OK`
```json
{
  "status": "ok"
}
```

---

### 2. Fuzzy Search

- **Path**: `GET /search`
- **Parameters**:
  - `q` (string, required): Query string (company name, acronym, SIREN, or SIRET).
  - `location` (string, optional): Postal code (e.g. `75001`), department (`75`), or COG commune code.
  - `naf` (string, optional): NAF activity code (e.g. `62.01Z`).
  - `dirigeant` (string, optional): Legal representative name.
  - `limit` (integer, optional, default: 10): Maximum results to return.
  - `page` (integer, optional, default: 1): Page index for pagination.

**Example Request**:
```bash
curl "http://localhost:8080/search?q=boucherie+dupan&location=75001&limit=2"
```

**Example Response**:
```json
{
  "total": 1,
  "page": 1,
  "page_size": 2,
  "total_pages": 1,
  "results": [
    {
      "siren": "732829320",
      "siret": "73282932000018",
      "denomination": "BOUCHERIE DUPONT",
      "sigle": "BD",
      "etat_administratif": "A",
      "code_commune": "75101",
      "code_postal": "75001",
      "code_naf": "47.22Z",
      "score": 0.945
    }
  ]
}
```

---

### 3. Direct SIREN Lookup

- **Path**: `GET /siren/{siren}`
- **Parameters**: `siren` (9 digits).

**Example Request**:
```bash
curl http://localhost:8080/siren/732829320
```

---

### 4. Direct SIRET Lookup

- **Path**: `GET /siret/{siret}`
- **Parameters**: `siret` (14 digits).

**Example Request**:
```bash
curl http://localhost:8080/siret/73282932000018
```
