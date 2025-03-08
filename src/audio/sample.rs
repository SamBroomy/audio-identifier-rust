use anyhow::Result;
use rodio::{Decoder, Source};
use std::{
    io::{Read, Seek},
    time::Duration,
};

pub struct BandpassFilterMonoSource {
    source: Box<dyn Source<Item = i16>>,
    target_sample_rate: u32,
    original_sample_rate: u32,
    downsample_ratio: usize,
    channels: u16,
    sample_counter: usize,
    // Filter states
    x1: f32,
    x2: f32, // Previous input values
    y1: f32,
    y2: f32, // Previous output values
    // Filter coefficients (calculated for bandpass between 20Hz-5kHz)
    hp_coef: f32, // High-pass filter coefficient (for 20Hz cutoff)
    lp_coef: f32, // Low-pass filter coefficient (for 5kHz cutoff)
}

impl BandpassFilterMonoSource {
    pub fn downsample<D>(data: D) -> Result<BandpassFilterMonoSource>
    where
        D: Read + Seek + Send + Sync + 'static,
    {
        let source = Box::new(Decoder::new(data).unwrap());
        // Apply bandpass filtering + downsampling to 11,025 Hz
        Ok(BandpassFilterMonoSource::new(source, 11025))
    }

    pub fn new(source: Box<dyn Source<Item = i16>>, target_sample_rate: u32) -> Self {
        let original_sample_rate = source.sample_rate();
        let channels = source.channels();
        // TODO: Fix as this will currently sample longer than the original source because it ignores the remainder of the division and should be sampling less often than it should.
        let downsample_ratio = (original_sample_rate / target_sample_rate) as usize;

        // Calculate filter coefficients based on RC filter design
        // High-pass filter (cutoff ~20Hz)
        let hp_coef = 0.98; // Simplified coefficient for 20Hz at 44.1kHz

        // Low-pass filter (cutoff ~5kHz)
        let lp_coef = 0.2; // Simplified coefficient for 5kHz at 44.1kHz

        BandpassFilterMonoSource {
            source,
            target_sample_rate,
            original_sample_rate,
            downsample_ratio,
            channels,
            sample_counter: 0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            hp_coef,
            lp_coef,
        }
    }

    // Apply bandpass filtering to a sample
    fn filter(&mut self, sample: i16) -> i16 {
        // Convert to float for filtering
        let input = sample as f32;

        // High-pass filter (removes frequencies below ~20Hz)
        let hp = self.hp_coef * (self.y1 + input - self.x1);
        self.x1 = input;
        self.y1 = hp;

        // Low-pass filter (removes frequencies above ~5kHz)
        let lp = self.lp_coef * hp + (1.0 - self.lp_coef) * self.y2;
        self.y2 = lp;

        // Convert back to i16
        lp as i16
    }
}

impl Iterator for BandpassFilterMonoSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // Skip directly to the next sample we need
        if self.channels == 1 {
            // Fast path for mono sources
            // Skip samples we don't need due to downsampling
            for _ in 0..self.downsample_ratio - 1 {
                // Skip samples without filtering them
                self.source.next()?;
            }
            // Process only the sample we'll actually use
            let sample = self.source.next()?;
            Some(self.filter(sample))
        } else {
            // Path for stereo or multi-channel sources
            // Skip samples we don't need
            for _ in 0..self.downsample_ratio - 1 {
                // Skip all channels for each sample position
                for _ in 0..self.channels {
                    self.source.next()?;
                }
            }

            // Process only the sample we'll use
            let mut sum = 0i32;
            for _ in 0..self.channels {
                sum += self.source.next()? as i32;
            }
            let averaged = (sum / self.channels as i32) as i16;

            // Filter and return the downsampled, mono sample
            Some(self.filter(averaged))
        }
    }
}

impl Source for BandpassFilterMonoSource {
    fn current_frame_len(&self) -> Option<usize> {
        self.source
            .current_frame_len()
            .map(|len| len / self.downsample_ratio / self.channels as usize)
    }

    fn channels(&self) -> u16 {
        1 // Mono output
        //self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.target_sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        // Calculate the new duration based on the downsample ratio

        self.source.total_duration()
    }
}
