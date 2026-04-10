/// Delta compression & pack engine using Zstd (faster than Git's zlib)

pub struct PackEngine;

impl PackEngine {
    /// Compress data using Zstd (level 3 = good balance)
    pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
        zstd::encode_all(data, 3)
            .map_err(|e| format!("Compression failed: {}", e))
    }

    /// Decompress Zstd data
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
        zstd::decode_all(data)
            .map_err(|e| format!("Decompression failed: {}", e))
    }

    /// Compute simple byte-level delta between base and target
    pub fn compute_delta(base: &[u8], target: &[u8]) -> Vec<DeltaOp> {
        let mut ops = Vec::new();
        let mut ti = 0;

        while ti < target.len() {
            // Try to find a matching chunk in base
            let mut best_offset = 0;
            let mut best_len = 0;

            for bi in 0..base.len() {
                let mut match_len = 0;
                while bi + match_len < base.len()
                    && ti + match_len < target.len()
                    && base[bi + match_len] == target[ti + match_len]
                {
                    match_len += 1;
                }
                if match_len > best_len && match_len >= 4 {
                    best_offset = bi;
                    best_len = match_len;
                }
            }

            if best_len >= 4 {
                ops.push(DeltaOp::Copy { offset: best_offset, len: best_len });
                ti += best_len;
            } else {
                // Collect consecutive inserts
                let start = ti;
                while ti < target.len() {
                    let mut found = false;
                    for bi in 0..base.len() {
                        let mut ml = 0;
                        while bi + ml < base.len()
                            && ti + ml < target.len()
                            && base[bi + ml] == target[ti + ml]
                        { ml += 1; }
                        if ml >= 4 { found = true; break; }
                    }
                    if found { break; }
                    ti += 1;
                }
                ops.push(DeltaOp::Insert(target[start..ti].to_vec()));
            }
        }

        ops
    }

    /// Apply delta operations to reconstruct target from base
    pub fn apply_delta(base: &[u8], ops: &[DeltaOp]) -> Vec<u8> {
        let mut result = Vec::new();
        for op in ops {
            match op {
                DeltaOp::Copy { offset, len } => {
                    result.extend_from_slice(&base[*offset..*offset + *len]);
                }
                DeltaOp::Insert(data) => {
                    result.extend_from_slice(data);
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub enum DeltaOp {
    Copy { offset: usize, len: usize },
    Insert(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let data = b"Hello ApexForge Git! This is a test of compression.";
        let compressed = PackEngine::compress(data).unwrap();
        let decompressed = PackEngine::decompress(&compressed).unwrap();
        assert_eq!(&decompressed, data);
    }

    #[test]
    fn test_delta_roundtrip() {
        let base = b"Hello World! This is line one.\nSecond line here.\n";
        let target = b"Hello World! This is line one.\nModified second line.\n";
        let delta = PackEngine::compute_delta(base, target);
        let reconstructed = PackEngine::apply_delta(base, &delta);
        assert_eq!(reconstructed, target);
    }
}
