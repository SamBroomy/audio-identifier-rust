use std::collections::BTreeMap;

use super::constellation::ConstellationPoint;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Fingerprint {
    pub hash: i64,
    pub time_offset: Decimal,
    pub confidence: Decimal,
    pub anchor_freq: Decimal,
    pub target_freq: Decimal,
    pub delta_t: Decimal,
}

impl Fingerprint {
    fn hash(anchor_freq: Decimal, target_freq: Decimal, delta_t: Decimal) -> i64 {
        // Convert to integers for hashing
        let a = quantize_frequency(anchor_freq).try_into().unwrap_or(0);
        let b = quantize_frequency(target_freq).try_into().unwrap_or(0);
        let dt = (delta_t * dec!(100)).try_into().unwrap_or(0);

        // FNV-1a hash algorithm
        const FNV_PRIME: u64 = 1099511628211;
        const FNV_OFFSET: u64 = 14695981039346656037;

        let mut hash = FNV_OFFSET;

        // Hash all three components
        hash ^= a;
        hash = hash.wrapping_mul(FNV_PRIME);

        hash ^= b;
        hash = hash.wrapping_mul(FNV_PRIME);

        hash ^= dt;
        hash = hash.wrapping_mul(FNV_PRIME);

        hash as i64
    }
}

fn confidence(magnitude1: Decimal, magnitude2: Decimal) -> Decimal {
    // Calculate confidence based on both magnitudes
    (magnitude1 * magnitude2).sqrt().unwrap_or(Decimal::ZERO)
}

impl From<(&ConstellationPoint, &ConstellationPoint)> for Fingerprint {
    fn from((anchor, target): (&ConstellationPoint, &ConstellationPoint)) -> Self {
        let delta_t = target.time - anchor.time;
        let hash = Self::hash(anchor.frequency, target.frequency, delta_t);
        let confidence = confidence(anchor.magnitude, target.magnitude);
        // Calculate confidence based on both magnitudes
        Fingerprint {
            hash,
            time_offset: anchor.time.round_dp(3),
            confidence,
            anchor_freq: anchor.frequency,
            target_freq: target.frequency,
            delta_t: delta_t.round_dp(3),
        }
    }
}

impl From<(i64, f64, i64, i64, i64, f64)> for Fingerprint {
    fn from(
        (hash, time_offset, confidence, anchor_freq, target_freq, delta_t): (
            i64,
            f64,
            i64,
            i64,
            i64,
            f64,
        ),
    ) -> Self {
        Fingerprint {
            hash,
            time_offset: time_offset.try_into().unwrap_or(Decimal::ZERO),
            confidence: confidence.into(),
            anchor_freq: anchor_freq.into(),
            target_freq: target_freq.into(),
            delta_t: delta_t.try_into().unwrap_or(Decimal::ZERO),
        }
    }
}
impl From<&Fingerprint> for (i64, f64, i64, i64, i64, f64) {
    fn from(fingerprint: &Fingerprint) -> Self {
        (
            fingerprint.hash,
            fingerprint.time_offset.try_into().unwrap_or(0.0),
            fingerprint.confidence.try_into().unwrap_or(0),
            fingerprint.anchor_freq.try_into().unwrap_or(0),
            fingerprint.target_freq.try_into().unwrap_or(0),
            fingerprint.delta_t.try_into().unwrap_or(0.0),
        )
    }
}

// Add this function to generate fingerprints
pub fn generate_fingerprints(points: BTreeMap<usize, Vec<ConstellationPoint>>) -> Vec<Fingerprint> {
    let mut fingerprints = Vec::new();
    let offsets = [1, 2, 3, 4, 5, 6, 8, 12];

    let chunk_indices: Vec<usize> = points.keys().cloned().collect();
    for current_chunk in chunk_indices {
        // Get anchor points from current chunk
        if !points.contains_key(&current_chunk) {
            continue;
        }
        let anchor_points = &points[&current_chunk];
        if anchor_points.is_empty() {
            continue;
        }
        let mut sorted_anchors = anchor_points.clone();
        sorted_anchors.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());

        // Take only the strongest 2 points from this chunk as anchors
        let filtered_anchors = sorted_anchors.iter().take(3);

        for anchor in filtered_anchors {
            let mut pair_count = 0;
            // Add more musically relevant offsets

            for offset in &offsets {
                // Apply weight to confidence score

                let target_chunk = current_chunk + offset;
                // Check if target chunk exists
                if !points.contains_key(&target_chunk) {
                    continue;
                }

                let target_points = &points[&target_chunk];
                if target_points.is_empty() {
                    continue;
                }

                // Find strongest peak in target chunk
                let target = target_points
                    .iter()
                    .max_by(|a, b| a.magnitude.partial_cmp(&b.magnitude).unwrap())
                    .unwrap();

                if !is_harmonically_related(anchor.frequency, target.frequency) {
                    continue;
                }
                let confidence = confidence(anchor.magnitude, target.magnitude);
                if confidence < dec!(40) {
                    continue;
                }

                let fingerprint = Fingerprint::from((anchor, target));
                fingerprints.push(fingerprint);
                pair_count += 1;

                // Limit number of fingerprints per anchor
                if pair_count >= 3 {
                    break;
                }
            }
        }
    }
    info!("Generated {} fingerprints", fingerprints.len());
    fingerprints
}

// Helper function to check if two frequencies have a meaningful musical relationship
fn is_harmonically_related(f1: Decimal, f2: Decimal) -> bool {
    // Common musical intervals (in frequency ratios)
    let ratios = [
        dec!(1.0),   // Unison
        dec!(1.125), // Major second (9/8)
        dec!(1.2),   // Minor third (6/5)
        dec!(1.25),  // Major third (5/4)
        dec!(1.333), // Perfect fourth (4/3)
        dec!(1.5),   // Perfect fifth (3/2)
        dec!(1.667), // Minor sixth (8/5)
        dec!(1.875), // Major seventh (15/8)
        dec!(2.0),   // Octave
    ];

    // Check if frequencies are related by any of the common ratios
    let ratio = if f1 > f2 { f1 / f2 } else { f2 / f1 };

    // Allow some tolerance (musical tuning isn't always perfect)
    for &r in &ratios {
        if (ratio - r).abs() < dec!(0.05) {
            return true;
        }
    }

    // Also check if they're in similar frequency bands
    (f2 - f1).abs() < dec!(20)
}

//f32 -> u32
// Adaptive frequency quantization
fn quantize_frequency(freq: Decimal) -> Decimal {
    if freq <= Decimal::ZERO {
        // Handle edge case
        Decimal::ZERO
    } else if freq < dec!(300) {
        // Fine resolution for bass frequencies (5Hz bins)
        (freq / dec!(5)).round()
    } else if freq < dec!(1000) {
        // Medium resolution for mid-range (10Hz bins)
        (freq / dec!(10)).round()
    } else {
        // Coarser resolution for high frequencies (20Hz bins)
        (freq / dec!(20)).round()
    }
}
