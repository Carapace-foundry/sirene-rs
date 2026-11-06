# Matching & Scoring

`sirene-rs` implements the official matching heuristics defined by the French State (DINUM *Annuaire des Entreprises*) and advanced fuzzy string metrics.

---

## 🧹 Text Normalization Pipeline

Before matching, every query and legal denomination is normalized:

1. **Unicode Diacritics Folding**: Accents (`é`, `è`, `ê`, `à`, `ç`) and ligatures (`œ` $\to$ `oe`, `æ` $\to$ `ae`) are mapped to ASCII equivalents without allocations if already pure ASCII.
2. **Case Folding & Punctuation**: Lowercased and stripped of special characters.
3. **DINUM Legal Form Stripping**: Words like `SARL`, `SAS`, `SA`, `EURL`, `SASU`, `EI` are ignored during distinctive token generation.
4. **Abbreviation Expansion**: Common administrative abbreviations are expanded:
   - `STE` $\to$ `SOCIETE`
   - `ETS` / `ETABL` $\to$ `ETABLISSEMENTS`
   - `CIE` $\to$ `COMPAGNIE`
   - `BTP` $\to$ `BATIMENT TRAVAUX PUBLICS`
5. **Stop-Word Removal**: French elisions (`d'`, `l'`, `qu'`) and conjunctions (`de`, `du`, `des`, `et`, `le`, `la`, `les`).

---

## 🔢 Validation of Numeric Identifiers (Luhn)

Queries containing 9-digit SIREN or 14-digit SIRET identifiers are validated instantly using bitwise Luhn arithmetic:
- **La Poste Derogation**: Fully supports La Poste's specific Luhn modulus-10 exception.
- **Short-Circuit**: If a query is a valid SIREN or SIRET, exact index lookup is executed first with a maximal score (1.0).

---

## 🎯 Composite Multi-Criteria Scoring

For fuzzy entity candidates, scores are computed according to:

$$\text{Score}_{\text{final}} = w_{\text{nom}} \cdot S_{\text{nom}} + w_{\text{loc}} \cdot S_{\text{loc}} + w_{\text{naf}} \cdot S_{\text{naf}} + w_{\text{dir}} \cdot S_{\text{dir}} + B_{\text{actif}}$$

### Metric Breakdown

- **$S_{\text{nom}}$ (Denomination Score)**: Blends Jaro-Winkler string prefix distance, Damerau-Levenshtein transposition distance, and unordered token overlap tolerance.
- **$S_{\text{loc}}$ (Location Score)**:
  - `1.0`: Exact match on INSEE COG commune code.
  - `0.8`: Exact match on postal code.
  - `0.5`: Department match (first 2 digits, with Corsica `2A`/`2B` handling).
  - `0.0`: Divergent location.
- **$S_{\text{naf}}$ (Activity Code)**:
  - `1.0`: Exact subclass match (e.g. `62.01Z`).
  - `0.6`: Division match (e.g. `62`).
- **$S_{\text{dir}}$ (RNE Dirigeants)**: Fuzzy distance on company executives and representatives.
- **$B_{\text{actif}}$ (Status Bonus)**: Boost for active companies; penalty for dissolved/ceased companies.
