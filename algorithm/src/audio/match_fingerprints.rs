use itertools::Itertools;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

use super::Fingerprint;

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub song_id: i64,
    pub confidence: f32,
    pub matched_count: usize,
    pub time_offset: f32, // How many seconds into the song the query starts
}

pub fn match_fingerprints(
    query_fingerprints: &[Fingerprint],
    potental_matches: HashMap<i64, Vec<Fingerprint>>,
) -> Vec<MatchResult> {
    let mut results = Vec::new();

    for (song_id, song_fingerprints) in potental_matches {
        // Create a hash map to track all the query hashes for fast lookup
        let query_hash_map: HashMap<i64, Vec<&Fingerprint>> = query_fingerprints
            .iter()
            .chunk_by(|fp| fp.hash)
            .into_iter()
            .map(|(hash, group)| (hash, group.collect()))
            .collect();

        // Track time offsets - the key insight of the Shazam algorithm
        let mut time_offsets = HashMap::new();
        let mut best_offset_count = 0;
        let mut best_offset = Decimal::ZERO;

        // For each fingerprint in the song
        for song_fp in &song_fingerprints {
            // Find matching query fingerprints with the same hash
            if let Some(matching_query_fps) = query_hash_map.get(&song_fp.hash) {
                for query_fp in matching_query_fps {
                    // Calculate time delta: how far into the song did our query start?
                    let offset = song_fp.time_offset - query_fp.time_offset;

                    // Round to nearest 0.1s to allow for small timing differences

                    let bucket = (offset * dec!(10)) / dec!(10);

                    // Count fingerprints with this offset
                    let count = time_offsets.entry(bucket).or_insert(0);
                    *count += 1;

                    // Track the best offset found
                    if *count > best_offset_count {
                        best_offset_count = *count;
                        best_offset = bucket;
                    }
                }
            }
        }

        // Calculate confidence
        let confidence = best_offset_count as f32 / query_fingerprints.len() as f32;

        // Only consider songs with reasonable match count
        if best_offset_count >= 3 && confidence > 0.05 {
            results.push(MatchResult {
                song_id,
                confidence,
                matched_count: best_offset_count,
                time_offset: best_offset.try_into().unwrap(),
            });
        }
    }

    // Sort results by confidence (best matches first)
    results.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}
