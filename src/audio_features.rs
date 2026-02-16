/*
 *  audio_features.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/14/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use super::mel_constants::{
    HANN_WINDOW, 
    MEL_BIN_STARTS,
    MEL_BIN_LENGTHS,
    MEL_BIN_OFFSETS,
    MEL_WEIGHTS,
};

use microfft::real::rfft_512;

const SAMPLE_RATE: usize = 16000;
const FRAME_LENGTH: usize = 512;
const FRAME_STEP: usize = 160;
const NUMBER_OF_FRAMES: usize = 97;
const NUMBER_OF_FFT_BINS: usize = 257;  // FRAME_LENGTH / 2 + 1
const NUMBER_OF_MEL_BINS: usize = 40;
const LOG_EPSILON: f32 = 1e-6;

pub fn convert_raw_to_float(
    raw_samples: &[u32; SAMPLE_RATE],
    output: &mut [f32; SAMPLE_RATE],
) {
    // Step 1: Convert each raw u32 to f32.
    // The SPH0645 outputs 18-bit signed data, 
    // Most Significant Bit-aligned in 32 bits.
    // Arithmetic right shift by 14 extracts the 18-bit value.
    // Then normalize to [-1.0, 1.0] by dividing by 131072.0 (2^17).
    for i in 0..SAMPLE_RATE {
        let signed_sample = (raw_samples[i] as i32) >> 14;
        output[i] = signed_sample as f32 / 131072.0;
    }

    // Step 2: Remove DC offset by subtracting the mean.
    // The SPH0645 has a significant DC bias that would
    // leak energy into low frequency bins.
    let sum: f32 = output.iter().copied().sum();
    let mean = sum / SAMPLE_RATE as f32;
    for sample in output.iter_mut() {
        *sample -= mean;
    }
}

pub fn extract_log_mel_spectrogram(
    audio: &[f32; SAMPLE_RATE],
    output: &mut [f32; NUMBER_OF_FRAMES * NUMBER_OF_MEL_BINS],
) {
    // Process each frame independently.
    for frame_index in 0..NUMBER_OF_FRAMES {
        let start = frame_index * FRAME_STEP;

        // --- Step A: Copy frame and apply Hann window ---
        let mut frame = [0.0f32; FRAME_LENGTH];
        for i in 0..FRAME_LENGTH {
            frame[i] = audio[start + i] * HANN_WINDOW[i];
        }

        // --- Step B: Compute 512-point real FFT ---
        // microfft::real::rfft_512 operates in-place on the frame buffer.
        // It returns a slice of 256 Complex32 values
        // (the positive half of the symmetric FFT output).
        let spectrum = rfft_512(&mut frame);

        // --- Step C: Compute magnitude spectrum ---
        // microfft packs:
        //  - DC (bin 0, 0 Hz) into spectrum[0].re;
        //  - Nyquist (bin 256, 8000 Hz, exactly half the sample rate)
        //    into spectrum[0].im.
        // Bins 1..255 are normal complex values.
        let mut magnitude = [0.0f32; NUMBER_OF_FFT_BINS];
        let dc_bin_value = spectrum[0].re;
        magnitude[0] = libm::fabsf(dc_bin_value);
        for i in 1..(FRAME_LENGTH / 2) {
            let real_portion = spectrum[i].re;
            let imaginary_portion = spectrum[i].im;
            magnitude[i] = libm::sqrtf((real_portion * real_portion) + (imaginary_portion * imaginary_portion));
        }
        let nyquist_bin_value = spectrum[0].im;
        magnitude[NUMBER_OF_FFT_BINS - 1] = libm::fabsf(nyquist_bin_value);

        // --- Step D: Apply sparse mel filterbank ---
        // For each mel bin, accumulate only the non-zero filter weights.
        let mel_offset = frame_index * NUMBER_OF_MEL_BINS;
        for j in 0..NUMBER_OF_MEL_BINS {
            let mut sum = 0.0f32;
            let start = MEL_BIN_STARTS[j];
            let offset = MEL_BIN_OFFSETS[j];
            let length = MEL_BIN_LENGTHS[j];
            for k in 0..length {
                sum += magnitude[start + k] * MEL_WEIGHTS[offset + k];
            }
            // --- Step E: Log scaling ---
            output[mel_offset + j] = libm::logf(sum + LOG_EPSILON);
        }
    }
}

pub fn quantize_features(
    features: &[f32; NUMBER_OF_FRAMES * NUMBER_OF_MEL_BINS],
    output: &mut [i8; NUMBER_OF_FRAMES * NUMBER_OF_MEL_BINS],
    scale: f32,
    zero_point: i32,
) {
    for i in 0..features.len() {
        // quantized = float_value / scale + zero_point
        let quantized_feature = (features[i] / scale) + zero_point as f32;

        // Clamp to int8 range [-128, 127] and convert.
        let clamped_quantized_feature = if quantized_feature < -128.0 {
            -128i8
        } else if quantized_feature > 127.0 {
            127i8
        } else {
            quantized_feature as i8
        };
        output[i] = clamped_quantized_feature;
    }
}