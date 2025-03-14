use itertools::Itertools;
use num_complex::Complex;
use rodio::Source;
use rust_decimal::{Decimal, MathematicalOps, prelude::ToPrimitive};
use rust_decimal_macros::dec;
use rustfft::{FftPlanner, num_traits::FromPrimitive};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use tracing::info;

pub use super::BandpassFilterMonoSource;

#[derive(Debug, Clone)]
pub struct ConstellationPoint {
    pub time: Decimal,      // Time in seconds
    pub frequency: Decimal, // Frequency in Hz
    pub magnitude: Decimal, // Magnitude of the peak
}

pub fn constellation_points(
    source: BandpassFilterMonoSource,
) -> BTreeMap<usize, Vec<ConstellationPoint>> {
    // Calculate chunk size in samples (for processed mono audio)
    let sample_rate = Decimal::from(source.sample_rate());
    let bytes_per_sample = 2; // i16 = 2 bytes
    let channels = source.channels() as usize;
    let chunk_size = 4096 * 2 / (bytes_per_sample * channels);

    // Create a Hamming window
    let hamming_window = (0..chunk_size)
        .map(|i| {
            // Hamming window function: 0.54 - 0.46 * cos(2π * n / (N-1))
            dec!(0.54)
                - dec!(0.46)
                    * (dec!(2.0) * Decimal::PI * Decimal::from(i) / Decimal::from(chunk_size - 1))
                        .cos()
            //0.54 - 0.46 * (2.0 * PI * i as f32 / (chunk_size - 1) as f32).cos()
        })
        .collect_vec();
    // Configure overlap
    let overlap_percent = 50; // 50% overlap between consecutive chunks
    let overlap_samples = (chunk_size * overlap_percent) / 100;
    let step_size = chunk_size - overlap_samples;
    info!(
        "Processing in chunks of {} samples with {}% overlap ({} samples)",
        chunk_size, overlap_percent, overlap_samples
    );
    let chunk_duration = Decimal::from(chunk_size) / sample_rate;
    let step_duration = Decimal::from(step_size) / sample_rate;

    info!(
        "Processing in chunks of {} samples ({:.3} seconds) with {}% overlap ({:.3} second steps)",
        chunk_size, chunk_duration, overlap_percent, step_duration
    );

    // Use a VecDeque to efficiently handle the sliding window of samples
    let mut sample_buffer = VecDeque::with_capacity(chunk_size * 2);
    let mut chunk = Vec::with_capacity(chunk_size);
    let mut fft_buffer = Vec::with_capacity(chunk_size);
    // Perform FFT on the windowed data for spectral analysis
    // This is where you would add frequency domain processing for fingerprinting

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(chunk_size);

    let mut constellation_points: BTreeMap<usize, Vec<ConstellationPoint>> = BTreeMap::new();
    let mut chunk_idx = 0_usize;

    // Process each sample
    for sample in source {
        sample_buffer.push_back(Decimal::from(sample));

        // When we've filled a chunk
        if sample_buffer.len() >= chunk_size {
            // Copy the chunk from the sample buffer
            chunk.clear();
            chunk.extend(sample_buffer.range(..chunk_size).cloned());

            // Process the chunk ---

            let current_chunk_size = Decimal::from(hamming_window.len().min(chunk.len()));
            let frequency_resolution = sample_rate / current_chunk_size;

            // Apply the Hamming window to the chunk in place
            apply_hamming_window(&mut chunk, &hamming_window);

            // Apply the FFT to the chunk
            apply_fft(&mut chunk, fft.clone(), &mut fft_buffer);

            let significant_peaks = significant_peaks(
                &chunk,
                frequency_resolution,
                Decimal::from(chunk_idx),
                Decimal::from(step_size),
                sample_rate,
            );

            constellation_points
                .entry(chunk_idx)
                .or_default()
                .extend(significant_peaks);

            // ------------------------
            // Remove step_size samples from the front (keeping the overlap portion)
            for _ in 0..step_size {
                sample_buffer.pop_front();
            }
            chunk_idx += 1;
        }
    }

    // After processing all chunks...
    info!(
        "Generated {} constellation points from {} chunks",
        constellation_points.values().flatten().count(),
        chunk_idx
    );
    constellation_points
}

fn apply_hamming_window(chunk: &mut [Decimal], hamming_window: &[Decimal]) {
    chunk
        .iter_mut()
        .zip(hamming_window.iter())
        .for_each(|(sample, &window)| {
            *sample *= window;
        });
}

fn apply_fft(
    chunk: &mut Vec<Decimal>,
    fft: Arc<dyn rustfft::Fft<f32>>,
    fft_buffer: &mut Vec<Complex<f32>>,
) {
    fft_buffer.clear();
    fft_buffer.extend(
        chunk
            .iter()
            .map(|&sample| Complex::new(sample.to_f32().unwrap(), 0.0)),
    );
    fft.process(fft_buffer);
    chunk.clear();
    chunk.extend(
        fft_buffer
            .iter()
            .map(|c| Decimal::from_f32(c.norm()).unwrap()),
    );
}

fn significant_peaks(
    chunk: &[Decimal],
    frequency_resolution: Decimal,
    index: Decimal,
    step_size: Decimal,
    sample_rate: Decimal,
) -> Vec<ConstellationPoint> {
    let max_magnitude = *chunk
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&Decimal::ZERO);

    chunk
        .windows(5)
        .enumerate()
        .filter_map(|(bin, window)| {
            if window[0] < window[1]
                && window[1] < window[2]
                && window[2] > window[3]
                && window[3] > window[4]
            {
                let freq = Decimal::from(bin) * frequency_resolution;
                // (frequency, magnitude)
                Some((freq, window[2]))
            } else {
                None
            }
        })
        .filter(|(freq, _)| (dec!(20)..=dec!(5000)).contains(freq))
        .sorted_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap())
        .take(4)
        .map(|(freq, magnitude)| {
            let time = index * step_size / sample_rate;
            let normalized_magnitude = (magnitude / max_magnitude) * dec!(100.0);
            ConstellationPoint {
                time,
                frequency: freq,
                magnitude: normalized_magnitude,
            }
        })
        .collect()
}
