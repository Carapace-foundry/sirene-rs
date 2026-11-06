#!/usr/bin/env python3
"""
Python Baseline Benchmark for French Corporate Entity Matching (SIRENE).
Executes the exact same query suite on the official INSEE SIRENE dataset (100,000 real enterprises)
to provide real, measured comparative figures against sirene-rs.
"""

import csv
import gzip
import os
import sys
import time
import multiprocessing as mp
from collections import defaultdict
from typing import List, Dict, Optional, Tuple

DOUBLED = [0, 2, 4, 6, 8, 1, 3, 5, 7, 9]

def get_process_rss_mb() -> float:
    """Reads Linux /proc/self/status VmRSS."""
    try:
        with open("/proc/self/status", "r") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    parts = line.split()
                    return float(parts[1]) / 1024.0
    except Exception:
        pass
    return 0.0

def validate_luhn_siren(s: str) -> bool:
    if len(s) != 9 or not s.isdigit():
        return False
    total = sum(DOUBLED[int(c)] if ((9 - 1 - i) & 1) == 1 else int(c) for i, c in enumerate(s))
    return total % 10 == 0

def normalize_text(text: str) -> str:
    # DINUM-like normalization: lowercase, diacritics folding, legal noise removal
    text = text.lower()
    for acc, rep in [('é', 'e'), ('è', 'e'), ('ê', 'e'), ('ë', 'e'), ('à', 'a'), ('â', 'a'),
                     ('î', 'i'), ('ï', 'i'), ('ô', 'o'), ('ö', 'o'), ('ù', 'u'), ('û', 'u'),
                     ('ç', 'c'), ('œ', 'oe'), ('æ', 'ae')]:
        text = text.replace(acc, rep)
    # Abbreviations
    text = text.replace('ste', 'societe').replace('ets', 'etablissements').replace('btp', 'batiment travaux publics')
    # Legal words and stop words
    words = text.replace('-', ' ').replace("'", ' ').replace('.', ' ').split()
    clean = [w for w in words if w not in ('sarl', 'sas', 'sasu', 'sa', 'eurl', 'sci', 'ei', 'de', 'du', 'des', 'et', 'le', 'la', 'les', 'en', 'par', 'pour')]
    return ' '.join(clean)

def get_trigrams(text: str) -> set:
    padded = f"  {text} "
    return {padded[i:i+3] for i in range(len(padded) - 2)}

def jaccard(s1: set, s2: set) -> float:
    if not s1 or not s2:
        return 0.0
    inter = len(s1 & s2)
    return inter / (len(s1) + len(s2) - inter)

class PythonSireneMatcher:
    def __init__(self, data_path: str):
        self.records: List[Dict] = []
        self.siren_index: Dict[str, int] = {}
        self.trigram_index: Dict[str, List[int]] = defaultdict(list)
        self.load(data_path)

    def load(self, data_path: str):
        t0 = time.time()
        with gzip.open(data_path, "rt", encoding="utf-8") as f:
            reader = csv.DictReader(f)
            for row in reader:
                siren = row["siren"]
                denom = row.get("denominationUniteLegale") or row.get("nomUniteLegale") or ""
                norm_denom = normalize_text(denom)
                doc_id = len(self.records)
                doc = {
                    "doc_id": doc_id,
                    "siren": siren,
                    "denom": denom,
                    "norm_denom": norm_denom,
                    "etat": row.get("etatAdministratifUniteLegale", "A"),
                    "naf": row.get("activitePrincipaleUniteLegale", ""),
                    "trigrams": get_trigrams(norm_denom),
                }
                self.records.append(doc)
                self.siren_index[siren] = doc_id
                for tri in doc["trigrams"]:
                    self.trigram_index[tri].append(doc_id)

        self.load_duration = time.time() - t0

    def search(self, query: str, top_k: int = 5):
        # 1. Exact SIREN short-circuit
        clean_q = query.strip().replace(" ", "")
        if validate_luhn_siren(clean_q) and clean_q in self.siren_index:
            doc = self.records[self.siren_index[clean_q]]
            return [(doc, 1.0)]

        # 2. Fuzzy text matching via Trigram Inverted Index
        norm_q = normalize_text(query)
        q_tris = get_trigrams(norm_q)
        if not q_tris:
            return []

        # Candidate accumulation
        candidate_counts: Dict[int, int] = defaultdict(int)
        for tri in q_tris:
            for doc_id in self.trigram_index.get(tri, []):
                candidate_counts[doc_id] += 1

        # Score top candidates
        candidates = []
        for doc_id, count in candidate_counts.items():
            doc = self.records[doc_id]
            sim = jaccard(q_tris, doc["trigrams"])
            if sim > 0.15:
                candidates.append((doc, sim))

        candidates.sort(key=lambda x: x[1], reverse=True)
        return candidates[:top_k]

# Global worker for multiprocessing
_matcher_instance = None
def _init_worker(data_path):
    global _matcher_instance
    _matcher_instance = PythonSireneMatcher(data_path)

def _worker_search(query):
    return _matcher_instance.search(query)

def run_benchmark():
    data_path = "data/real_stock_unite_legale_100k.csv.gz"

    if not os.path.exists(data_path):
        print(f"Error: {data_path} not found.")
        sys.exit(1)

    print("=======================================================")
    print(" 🐍 PYTHON BASELINE BENCHMARK (REAL INSEE SIRENE 100K) ")
    print("=======================================================")

    t_start_load = time.time()
    matcher = PythonSireneMatcher(data_path)
    cold_startup_ms = (time.time() - t_start_load) * 1000.0
    ram_rss_mb = get_process_rss_mb()

    print(f"📁 Real Dataset: {len(matcher.records):,} official French enterprises")
    print(f"⏱️  Cold startup / load time: {cold_startup_ms:,.2f} ms")
    print(f"🧠 Python Process RAM (RSS):  {ram_rss_mb:.2f} MB")

    queries = [
        # Exact real enterprise names
        "ETABLISSEMENTS LUCIEN BIQUEZ",
        "SOCIETE DES SUCRERIES DU MARQUENTERRE",
        "SOCIETE BRETONNE DE TRAVAUX PUBLICS",
        "SOCIETE DE DISTRIBUTION DE PAPIER",
        "HERNAS CARTONNAGE",
        "ETABLISSEMENTS ADRIEN RIQUIER",
        "ETABLISSEMENTS BOZZI",
        "MONCHAUX WINDERS",
        "BOULANGERIE BOURGOGNE",
        "BOULANGERIE PANI",
        "PHARMACIE CARE",
        "TRANSPORTS MORAND FRIGORIFIQUES",
        # Typos / Fuzzy queries
        "ets lusien bikez",
        "ste bretonne btp",
        "sucreries marquenterre",
        "distribution papier",
        "hernas carton",
        "transport morand frigo",
        "boulangrie bourgogne",
        "farmacie care",
        # Prefix / Autocomplete
        "boulang",
        "transpor",
        "pharmaci",
        "etablisse",
        # Exact SIREN lookups
        "005420021",
        "005420120",
        "005480082",
        "005480546",
        "005520176",
        "005520325",
        "005780051",
        "006480065",
        "017350034",
    ]

    # Warm-up
    for q in queries[:5]:
        matcher.search(q)

    # Single-thread Latency Profile (5,000 queries)
    target_queries = 5000
    print(f"\n📊 Measuring single-thread latency profile ({target_queries:,} queries)...")
    latencies = []
    t_run_start = time.time()

    for i in range(target_queries):
        q = queries[i % len(queries)]
        t0 = time.time()
        _ = matcher.search(q)
        latencies.append((time.time() - t0) * 1000.0)

    duration_secs = time.time() - t_run_start
    single_thread_qps = target_queries / duration_secs

    latencies.sort()
    n = len(latencies)
    p50_ms = latencies[int(n * 0.50)]
    p90_ms = latencies[int(n * 0.90)]
    p95_ms = latencies[int(n * 0.95)]
    p99_ms = latencies[int(n * 0.99)]
    mean_ms = sum(latencies) / n
    min_ms = latencies[0]
    max_ms = latencies[-1]

    print(f"   p50 (median):  {p50_ms:.3f} ms")
    print(f"   p90:           {p90_ms:.3f} ms")
    print(f"   p95:           {p95_ms:.3f} ms")
    print(f"   p99:           {p99_ms:.3f} ms")
    print(f"   Mean:          {mean_ms:.3f} ms")
    print(f"   Throughput 1T: {single_thread_qps:.0f} req/s")

    # Multi-process throughput saturation
    num_cpus = mp.cpu_count()
    print(f"\n🔥 Measuring multi-process throughput ({num_cpus} worker processes, 5,000 queries)...")
    batch_queries = [queries[i % len(queries)] for i in range(target_queries)]

    t_mp_start = time.time()
    with mp.Pool(num_cpus, initializer=_init_worker, initargs=(data_path,)) as pool:
        pool.map(_worker_search, batch_queries, chunksize=50)
    mp_duration = time.time() - t_mp_start
    multi_process_qps = target_queries / mp_duration
    print(f"   Multi-process Throughput: {multi_process_qps:.0f} req/s (across {num_cpus} cores)")

    # Exact SIREN Lookups (20,000 lookups)
    siren_keys = ["005420021", "005420120", "005480082", "005480546", "005520176", "005520325", "005780051", "006480065"]
    print("\n🎯 Measuring exact SIREN lookups (20,000 lookups)...")
    siren_latencies = []
    t_siren_start = time.time()
    for i in range(20_000):
        s = siren_keys[i % len(siren_keys)]
        t0 = time.time()
        is_valid = validate_luhn_siren(s)
        _ = matcher.siren_index.get(s) if is_valid else None
        siren_latencies.append((time.time() - t0) * 1000.0)
    siren_duration = time.time() - t_siren_start
    siren_latencies.sort()
    siren_p50 = siren_latencies[int(len(siren_latencies) * 0.50)]
    siren_qps = 20_000 / siren_duration
    print(f"   SIREN Lookup p50:  {siren_p50:.4f} ms")
    print(f"   SIREN Throughput:  {siren_qps:,.0f} req/s")

    # Batch CSV Matching Throughput
    print("\n🔄 Measuring batch CSV reconciliation throughput...")
    test_csv_rows = queries[:10] * 100 # 1,000 records
    t_batch_start = time.time()
    for q in test_csv_rows:
        _ = matcher.search(q, top_k=1)
    batch_duration = time.time() - t_batch_start
    batch_qps = len(test_csv_rows) / batch_duration
    print(f"   Batch Reconciliation: {batch_qps:.0f} records/s")

    print("\n=======================================================")
    print(" 📊 PYTHON BASELINE BENCHMARK SUMMARY (REAL DATASET)  ")
    print("=======================================================")
    print(f"| Cold Startup:      {cold_startup_ms:.1f} ms")
    print(f"| Process RAM:       {ram_rss_mb:.1f} MB")
    print(f"| Search p50:        {p50_ms:.3f} ms")
    print(f"| Search p90:        {p90_ms:.3f} ms")
    print(f"| Search p99:        {p99_ms:.3f} ms")
    print(f"| Single-Thread QPS: {single_thread_qps:.0f} req/s")
    print(f"| Multi-Process QPS: {multi_process_qps:.0f} req/s")
    print(f"| SIREN Lookup p50:  {siren_p50:.4f} ms")
    print(f"| Batch Linkage QPS: {batch_qps:.0f} rec/s")

if __name__ == "__main__":
    run_benchmark()
