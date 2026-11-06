use flate2::read::{DeflateDecoder, GzDecoder};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Opens a transparent streaming reader for `.csv`, `.gz`, or `.zip` files.
///
/// For `.zip` files, this performs true streaming decompression without buffering
/// the entire uncompressed CSV file in memory, protecting against Out-Of-Memory (OOM)
/// crashes on multi-gigabyte official INSEE datasets and Zip Bomb vulnerabilities.
pub fn open_streaming_reader(path: &Path) -> Result<Box<dyn Read + Send>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "gz" => {
            let decoder = GzDecoder::new(file);
            Ok(Box::new(BufReader::new(decoder)))
        }
        "zip" => {
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| format!("Invalid ZIP archive {}: {}", path.display(), e))?;

            if archive.is_empty() {
                return Err(format!("ZIP archive '{}' is empty", path.display()));
            }

            // Find first CSV entry in archive (or default to first entry)
            let mut target_idx = None;
            for i in 0..archive.len() {
                if let Ok(entry) = archive.by_index(i) {
                    if let Ok(name) = entry.name() {
                        if name.to_ascii_lowercase().ends_with(".csv") {
                            target_idx = Some(i);
                            break;
                        }
                    }
                }
            }

            let idx = target_idx.unwrap_or(0);
            let (data_start, compressed_size, compression_method) = {
                let entry = archive.by_index(idx).map_err(|e| {
                    format!("Cannot read file inside ZIP {}: {}", path.display(), e)
                })?;
                let start = entry.data_start().ok_or_else(|| {
                    format!(
                        "Cannot determine data start offset for ZIP entry '{}'",
                        path.display()
                    )
                })?;
                (start, entry.compressed_size(), entry.compression())
            };

            // Drop archive to release file lock, then open a direct streaming reader
            drop(archive);

            let mut raw_file = File::open(path)
                .map_err(|e| format!("Failed to reopen {}: {}", path.display(), e))?;
            raw_file.seek(SeekFrom::Start(data_start)).map_err(|e| {
                format!("Failed to seek to ZIP entry data at {}: {}", data_start, e)
            })?;

            let bounded_raw = raw_file.take(compressed_size);

            match compression_method {
                zip::CompressionMethod::Deflated => {
                    let decoder = DeflateDecoder::new(bounded_raw);
                    Ok(Box::new(BufReader::with_capacity(128 * 1024, decoder)))
                }
                zip::CompressionMethod::Stored => {
                    Ok(Box::new(BufReader::with_capacity(128 * 1024, bounded_raw)))
                }
                other => Err(format!(
                    "Unsupported ZIP compression method {:?} for entry in {}",
                    other,
                    path.display()
                )),
            }
        }
        _ => {
            // Default plain CSV / text
            Ok(Box::new(BufReader::with_capacity(128 * 1024, file)))
        }
    }
}
