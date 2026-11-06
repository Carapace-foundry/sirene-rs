#!/usr/bin/env python3
"""
Downloads official INSEE Base SIRENE datasets from data.gouv.fr
and extracts real enterprise subsets for benchmarking and testing.
"""

import argparse
import csv
import gzip
import io
import os
import sys
import urllib.request
import zipfile

DATA_GOUV_API_URL = "https://www.data.gouv.fr/api/1/datasets/base-sirene-des-entreprises-et-de-leurs-etablissements-siren-siret/"

def find_stock_unite_legale_url() -> str:
    """Discovers latest official StockUniteLegale URL from data.gouv.fr API."""
    req = urllib.request.Request(DATA_GOUV_API_URL, headers={"User-Agent": "sirene-rs/1.0"})
    import json
    with urllib.request.urlopen(req) as resp:
        data = json.load(resp)
        for r in data.get("resources", []):
            title = r.get("title", "")
            if "StockUniteLegale" in title and r.get("format", "").lower() == "zip":
                return r.get("url")
    # Fallback to standard mirror
    return "https://static.data.gouv.fr/resources/base-sirene-des-entreprises-et-de-leurs-etablissements-siren-siret/20261103-091933/stock-stockunitelegale-csv.zip"

def extract_real_subset(zip_path: str, output_path: str, limit: int = 100_000, active_only: bool = True):
    print(f"Extracting {limit:,} real enterprises from {zip_path} -> {output_path}...")
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)

    with zipfile.ZipFile(zip_path) as z:
        csv_filename = [n for n in z.namelist() if n.endswith(".csv")][0]
        with z.open(csv_filename) as raw_stream, \
             gzip.open(output_path, "wt", encoding="utf-8", newline="") as out_gz:
            
            text_stream = io.TextIOWrapper(raw_stream, encoding="utf-8")
            reader = csv.reader(text_stream)
            writer = csv.writer(out_gz)

            header = next(reader)
            writer.writerow(header)

            etat_idx = header.index("etatAdministratifUniteLegale") if "etatAdministratifUniteLegale" in header else 20
            denom_idx = header.index("denominationUniteLegale") if "denominationUniteLegale" in header else 23
            nom_idx = header.index("nomUniteLegale") if "nomUniteLegale" in header else 21

            count = 0
            for row in reader:
                if active_only and len(row) > etat_idx and row[etat_idx] != "A":
                    continue
                denom = row[denom_idx] if len(row) > denom_idx else ""
                nom = row[nom_idx] if len(row) > nom_idx else ""
                if denom.strip() or nom.strip():
                    writer.writerow(row)
                    count += 1
                    if count >= limit:
                        break

    print(f"Successfully extracted {count:,} real enterprises into {output_path}.")

def main():
    parser = argparse.ArgumentParser(description="Download and extract official INSEE SIRENE data.")
    parser.add_argument("--limit", type=int, default=100_000, help="Number of records to extract")
    parser.add_argument("--output", type=str, default="data/real_stock_unite_legale_100k.csv.gz", help="Output file")
    args = parser.parse_args()

    zip_tmp = "data/StockUniteLegale_tmp.zip"
    url = find_stock_unite_legale_url()
    print(f"Downloading from official source: {url}...")
    urllib.request.urlretrieve(url, zip_tmp)
    try:
        extract_real_subset(zip_tmp, args.output, limit=args.limit)
    finally:
        if os.path.exists(zip_tmp):
            os.remove(zip_tmp)

if __name__ == "__main__":
    main()

