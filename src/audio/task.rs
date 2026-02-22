/*
 *  audio/task.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/14/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use cortex_m::singleton;
use defmt::*;
use pio::pio_asm;
use rp235x_hal as hal;
use hal::dma::double_buffer;
use hal::pac::PIO0;
use hal::pio::{
    PIO,
    SM0,
    UninitStateMachine,
};

use super::features;

use crate::detectable_words::DetectableWord;
use crate::hardware_resources::FunctionPioPin;
use crate::output;
use crate::inference::tflite;

const INPUT_QUANTIZATION_SCALE: f32 = 0.07350979000329971;
const INPUT_QUANTIZATION_ZERO_POINT: i32 = 60;
const OUTPUT_SCALE: f32 = 0.00390625;
const OUTPUT_ZERO_POINT: i32 = -128;

pub struct MicrophoneResources {

    // Pins.
    pub bclk_pin: FunctionPioPin,
    pub lrcl_pin: FunctionPioPin,
    pub dout_pin: FunctionPioPin,

    // Timer for measuring feature extraction and inference durations.
    pub timer: hal::Timer<hal::timer::CopyableTimer1>,

    // Programmable I/O (PIO).
    pub pio: PIO<PIO0>,
    pub unitialized_state_machine: UninitStateMachine<(PIO0, SM0)>,

    // Direct memory access (DMA).
    pub dma_channels: hal::dma::Channels,

}

pub fn run(microphone_resources: MicrophoneResources) -> ! {

    let MicrophoneResources {
        bclk_pin,
        lrcl_pin,
        dout_pin,
        timer: _timer,
        mut pio,
        unitialized_state_machine,
        dma_channels,
    } = microphone_resources;

    // The PIO program generates the I2S clock signals (BCLK and LRCL) 
    // via side-set while simultaneously reading data bits
    // via the in instruction.
    #[rustfmt::skip]
    let i2s_program = pio_asm!(
        ".side_set 2",
        //
        // Side-set controls 2 pins:
        //  - bit 1 = LRCL (word select), a.k.a. WS;
        //  - bit 0 = BCLK.
        // So side 0bWB where W=word select, B=bit clock.
        //
        // SPH0645 outputs data on the FALLING edge of BCLK,
        // so we sample data when BCLK is LOW (side 0bx0).
        //
        // Left channel (WS=0): read 32 bits into the ISR (Input Shift Register).
        "    set x, 30          side 0b01", // BCLK high, WS=0; x = 30 (loop 31 times)
        "left_data:",
        "    in pins, 1         side 0b00", // BCLK falls -> sample 1 data bit
        "    jmp x-- left_data  side 0b01", // BCLK rises -> loop
        "    in pins, 1         side 0b10", // BCLK falls, WS goes high (right channel)
        //
        // Right channel (WS=1): skip 32 bits (our microphone only outputs on the left).
        "    set x, 30          side 0b11", // BCLK rises, WS=1; x = 30 (loop 31 times)
        "right_skip:",
        "    nop                side 0b10", // BCLK falls -> ignore data
        "    jmp x-- right_skip side 0b11", // BCLK rises -> loop
        "    nop                side 0b00", // BCLK falls, WS goes back to 0 (left)
    );

    // Calculate clock divider.
    //
    // Each bit takes 2 PIO instructions (sample + jump), 
    // so the PIO clock must run at 2x the BCLK frequency.
    //
    // BCLK = 16000 * 32 * 2 = 1,024,000 Hz
    // PIO clock = BCLK * 2 = 2,048,000 Hz (2 instructions per bit)
    // System clock on RP2350 = 150 MHz
    // Divider = 150,000,000 / 2,048,000 = 73.2421875
    // Fraction (remainder): 0.2421875 * 256 ≈ 62
    let divider_int: u16 = 73;
    let divider_frac: u8 = 62;

    // Install the program into PIO instruction memory.
    let installed_program = pio.install(&i2s_program.program).unwrap();

    // Setup the PIO state machine.
    let (mut state_machine, rx, _tx) = 
        hal::pio::PIOBuilder::from_installed_program(installed_program)
            .in_pin_base(dout_pin.index)
            .side_set_pin_base(bclk_pin.index)
            // I2S transmits the most significant bit first.
            // So when the SPH0645 sends an audio sample, the first bit
            // on the wire is the highest-value bit, then the next-highest, 
            // and so on.
            // Hence, we need to use the `Left` as the shift direction for PIO.
            .in_shift_direction(hal::pio::ShiftDirection::Left)
            // After 32 bits are shifted into the ISR (Input Shift Register), 
            // it’s automatically pushed to the RX FIFO
            // without any PIO instruction needed.
            .autopush(true)
            .push_threshold(32)
            .clock_divisor_fixed_point(divider_int, divider_frac)
            // Join both TX and RX FIFOs into a single 8-entry deep RX FIFO.
            .buffers(hal::pio::Buffers::OnlyRx)
            .build(unitialized_state_machine);

    // Set pin directions.
    state_machine.set_pindirs([
        (dout_pin.index, hal::pio::PinDir::Input),
        (bclk_pin.index, hal::pio::PinDir::Output),
        (lrcl_pin.index, hal::pio::PinDir::Output),
    ]);
    
    // Start the PIO state machine.
    state_machine.start();

    // Create two buffers for a Double-buffer DMA setup.
    // These buffers can hold 250ms of audio, i.e. 4,000 samples (16 KB).
    const DMA_BUFFER_SAMPLES: usize = 4000;
    let buffer_a = singleton!(: [u32; DMA_BUFFER_SAMPLES] = [0u32; DMA_BUFFER_SAMPLES]).unwrap();
    let buffer_b = singleton!(: [u32; DMA_BUFFER_SAMPLES] = [0u32; DMA_BUFFER_SAMPLES]).unwrap();

    // Sliding window: 1 second of raw audio (16,000 samples).
    // Each DMA cycle appends 4,000 new samples, shifting out the oldest 4,000.
    const AUDIO_WINDOW_SAMPLES: usize = 16000;
    let audio_window = singleton!(: [u32; AUDIO_WINDOW_SAMPLES] = [0u32; AUDIO_WINDOW_SAMPLES]).unwrap();

    let float_audio = singleton!(: [f32; 16000] = [0.0f32; 16000]).unwrap();
    let mel_features = singleton!(: [f32; 97 * 40] = [0.0f32; 97 * 40]).unwrap();
    let quantized_features = singleton!(: [i8; 97 * 40] = [0i8; 97 * 40]).unwrap();
    let tensor_arena = singleton!(: [u8; 192 * 1024] = [0u8; 192 * 1024]).unwrap();

    const MODEL_DATA: &[u8] = include_bytes!(
        "../../spoken-word-detection-model/models/model_quantized.tflite"
    );

    let model_initialization_status = unsafe {
        tflite::tflite_model_init(
            MODEL_DATA.as_ptr(), 
            tensor_arena.as_mut_ptr(), 
            tensor_arena.len(),
        )
    };
    if model_initialization_status != tflite::Status::Ok {
        defmt::panic!("TFLite init failed: {:?}", model_initialization_status);
    }

    let arena_used = unsafe { tflite::tflite_model_arena_used() };
    info!(
        "TFLite model loaded. Tensor arena: {} / {} bytes",
        arena_used,
        tensor_arena.len(),
    );

    // Start filling buffer_a using channels 0 and 1.
    let dma_configuration = double_buffer::Config::new(
        (dma_channels.ch0, dma_channels.ch1), 
        rx,
        buffer_a,
    );
    let dma_transfer = dma_configuration.start();

    // Queue buffer_b as the next destination.
    // When buffer_a is full, DMA automaticaly chains to the second channel
    // and starts filling buffer_b.
    let mut dma_transfer = dma_transfer.write_next(buffer_b);

    loop {
        // Wait for the active buffer to be completely filled.
        // During this wait, the CPU is free.
        let (
            filled_buffer, 
            next_dma_transfer,
        ) = dma_transfer.wait();

        // Filled buffer now contains 4,000 raw samples.
        // DMA is already filling the other buffer in the background.

        // Shift the sliding window left by 4,000 samples,
        // discarding the oldest quarter and making room for the new samples.
        audio_window.copy_within(DMA_BUFFER_SAMPLES.., 0);

        // Append the new 4,000 samples at the end.
        audio_window[(AUDIO_WINDOW_SAMPLES - DMA_BUFFER_SAMPLES)..]
            .copy_from_slice(filled_buffer);

        // Convert raw I2S samples to normalized float audio.
        features::convert_raw_to_float(audio_window, float_audio);

        // Extract log mel spectrogram (97 frames * 40 mel bins).
        features::extract_log_mel_spectrogram(float_audio, mel_features);

        // Quantize to uint8 for the TFLite model.
        features::quantize_features(
            mel_features,
            quantized_features,
            INPUT_QUANTIZATION_SCALE,
            INPUT_QUANTIZATION_ZERO_POINT,
        );

        // Copy quantized features into the model’s input tensor.
        let input_ptr = unsafe { tflite::tflite_model_input_data() };
        let input_size = unsafe { tflite::tflite_model_input_size() };
        unsafe {
            core::ptr::copy_nonoverlapping(
                quantized_features.as_ptr(), 
                input_ptr, 
                input_size,
            );
        }

        // Run inference.
        let inference_status = unsafe { tflite::tflite_model_invoke() };
        if inference_status != tflite::Status::Ok {
            error!("Inference failed: {:?}", inference_status);
        }

        // Read output (12 uint8 values).
        let output_ptr = unsafe { tflite::tflite_model_output_data() };
        let output_size = unsafe { tflite::tflite_model_output_size() };
        let output = unsafe {
            core::slice::from_raw_parts(
                output_ptr, 
                output_size,
            )
        };

        // Find the index of maximum probability and keep track of its value.
        let mut max_index = 0usize;
        let mut max_value = i8::MIN;
        for (i, &value) in output.iter().enumerate() {
            if value > max_value {
                max_value = value;
                max_index = i;
            }
        }

        // Convert `max_index` to an actual `DetectedWord`.`
        let detected_word = DetectableWord::try_from(max_index)
            .unwrap_or(DetectableWord::Unknown);

        // Calculate confidence level.
        let confidence = ((max_value as f32) - (OUTPUT_ZERO_POINT as f32)) * OUTPUT_SCALE;

        // Pass the detected word and the confidence level to the output controller.
        output::controller::handle_detected_word(detected_word, confidence);

        // Give the processed buffer back to for the next cycle.
        dma_transfer = next_dma_transfer.write_next(filled_buffer);

    }
}