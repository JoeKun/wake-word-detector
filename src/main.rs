/*
 *  main.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/14/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

#![no_std]
#![no_main]

mod audio_features;
mod mel_constants;
mod tflite;

use defmt::*;
use defmt_rtt as _;
// use embedded_hal::delay::DelayNs;
// use embedded_hal::digital::OutputPin;
#[cfg(target_arch = "riscv32")]
use panic_halt as _;
#[cfg(target_arch = "arm")]
use panic_probe as _;

// Alias for the HAL crate needed for Raspberry Pi Pico 2.
#[cfg(rp2350)]
use rp235x_hal as hal;

// Alias for the HAL crate needed for Raspberry Pi Pico.
#[cfg(rp2040)]
use rp2040_hal as hal;

use cortex_m::singleton;
use hal::dma::{
    DMAExt,
    double_buffer,
};
use hal::gpio::FunctionPio0;
use hal::gpio::Pin;
use hal::pio::PIOExt;
use pio::pio_asm;

/// The linker will place this boot block at the start of our program image.
/// We need this to help the ROM bootloader get our code up and running.
#[unsafe(link_section = ".boot2")]
#[used]
#[cfg(rp2040)]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

/// Tell the Boot ROM about our application.
#[unsafe(link_section = ".start_block")]
#[used]
#[cfg(rp2350)]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

const INPUT_QUANTIZATION_SCALE: f32 = 0.07350979000329971;
const INPUT_QUANTIZATION_ZERO_POINT: i32 = 60;
const OUTPUT_SCALE: f32 = 0.00390625;
const OUTPUT_ZERO_POINT: i32 = -128;

const CLASS_NAMES: [&str; 12] = [
    "six", 
    "seven", 
    "up", 
    "down", 
    "right", 
    "left",
    "on", 
    "off", 
    "wow", 
    "happy", 
    "unknown", 
    "silence",
];

/// Entry point to our bare-metal application.
///
/// The `#[hal::entry]` macro ensures the Cortex-M start-up code
/// calls this function as soon as all global variables
/// and the spinlock are initialized.
///
/// The function configures the rp2040 and rp235x peripherals,
/// then toggles a GPIO pin in an infinite loop.
/// If there’s an LED connected to that pin, it will blink.
#[hal::entry]
fn main() -> ! {
    info!("wake-word-detector start.");
    
    // Grab our peripheral access crate.
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Setup the watchdog driver, which is needed by the clock setup code.
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks.
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    #[cfg(rp2040)]
    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    #[cfg(rp2350)]
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // The single-cycle I/O block controls our GPIO pins.
    let sio = hal::Sio::new(pac.SIO);

    // Split the DMA peripheral.
    let dma = pac.DMA.split(&mut pac.RESETS);

    // Set the pins to their default state.
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Begin setup of SPH0645 microphone breakout.
    // Reassign GP13, GP14, and GP15 from their default GPIO function to PIO0.
    let _bclk_pin: Pin<_, FunctionPio0, _> = pins.gpio13.into_function();
    let _lrcl_pin: Pin<_, FunctionPio0, _> = pins.gpio14.into_function();
    let _dout_pin: Pin<_, FunctionPio0, _> = pins.gpio15.into_function();

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
    // TODO: joel!! adapt this for RP2040
    let divider_int: u16 = 73;
    let divider_frac: u8 = 62;

    // Split PIO0 into the controller and 4 state machines.
    let (
        mut pio, 
        unitialized_state_machine_0, 
        _unitialized_state_machine_1, 
        _unitialized_state_machine_2, 
        _unitialized_state_machine_3
    ) = pac.PIO0.split(&mut pac.RESETS);

    // Install the program into PIO instruction memory.
    let installed_program = pio.install(&i2s_program.program).unwrap();

    // Setup the PIO state machine.
    let (mut state_machine, rx, _tx) = 
        hal::pio::PIOBuilder::from_installed_program(installed_program)
            // DOUT = GP15
            .in_pin_base(15)
            // BCLK = GP13, LRCL = GP14 (consecutive)
            .side_set_pin_base(13)
            // I2S transmits the most significant bit first.
            // So when the SPH0645 sends an audio sample, the first bit
            // on the wire is the highest-value bit, then the next-highest, 
            // and so on.
            // Hence, we need to use the `Left` as the shift direction for PIO.
            .in_shift_direction(hal::pio::ShiftDirection::Left)
            // After 32 bits are shifted into the ISR (Input Shift Register), 
            // it's automatically pushed to the RX FIFO
            // without any PIO instruction needed.
            .autopush(true)
            .push_threshold(32)
            .clock_divisor_fixed_point(divider_int, divider_frac)
            // Join both TX and RX FIFOs into a single 8-entry deep RX FIFO.
            .buffers(hal::pio::Buffers::OnlyRx)
            .build(unitialized_state_machine_0);

    // Set pin directions.
    state_machine.set_pindirs([
        (15, hal::pio::PinDir::Input),
        (13, hal::pio::PinDir::Output),
        (14, hal::pio::PinDir::Output),
    ]);
    
    // Start the PIO state machine.
    state_machine.start();

    // Create two buffers for a Double-buffer DMA setup.
    // These buffers can hold 1 second of audio, i.e. 16,000 samples (64 KB).
    let buffer_a = singleton!(: [u32; 16000] = [0u32; 16000]).unwrap();
    let buffer_b = singleton!(: [u32; 16000] = [0u32; 16000]).unwrap();

    let float_audio = singleton!(: [f32; 16000] = [0.0f32; 16000]).unwrap();
    let mel_features = singleton!(: [f32; 97 * 40] = [0.0f32; 97 * 40]).unwrap();
    let quantized_features = singleton!(: [i8; 97 * 40] = [0i8; 97 * 40]).unwrap();
    let tensor_arena = singleton!(: [u8; 192 * 1024] = [0u8; 192 * 1024]).unwrap();

    const MODEL_DATA: &[u8] = include_bytes!(
        "../spoken-word-detection-model/models/model_quantized.tflite"
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
        (dma.ch0, dma.ch1), 
        rx, 
        buffer_a, 
    );
    let dma_transfer = dma_configuration.start();

    // Queue buffer_b as the next destination.
    // When buffer_a is full, DMA automaticaly chains to the second channel
    // and starts filling buffer_b.
    let mut dma_transfer = dma_transfer.write_next(buffer_b);

    // Configure GP25 as an output.
    // let mut led_pin = pins.gpio25.into_push_pull_output();
    loop {
        // Wait for the active buffer to be completely filled.
        // During this wait, the CPU is free.
        let (
            filled_buffer, 
            next_dma_transfer,
        ) = dma_transfer.wait();

        // Filled buffer now contains 16,000 raw samples.
        // DMA is already filling the other buffer in the background.

        let t0 = timer.get_counter();

        // Convert raw I2S samples to normalized float audio.
        audio_features::convert_raw_to_float(filled_buffer, float_audio);

        // Extract log mel spectrogram (97 frames * 40 mel bins).
        audio_features::extract_log_mel_spectrogram(float_audio, mel_features);

        // Quantize to uint8 for the TFLite model.
        audio_features::quantize_features(
            mel_features,
            quantized_features,
            INPUT_QUANTIZATION_SCALE,
            INPUT_QUANTIZATION_ZERO_POINT,
        );

        let t1 = timer.get_counter();

        // Copy quantized features into the model's input tensor.
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

        let t2 = timer.get_counter();
        info!(
            "Timing: features={}ms, inference={}ms",
            (t1 - t0).to_millis(),
            (t2 - t1).to_millis(),
        );

        // Read output (12 uint8 values).
        let output_ptr = unsafe { tflite::tflite_model_output_data() };
        let output_size = unsafe { tflite::tflite_model_output_size() };
        let output = unsafe {
            core::slice::from_raw_parts(
                output_ptr, 
                output_size,
            )
        };

        let mut max_index = 0usize;
        let mut max_value = i8::MIN;
        for (i, &value) in output.iter().enumerate() {
            if value > max_value {
                max_value = value;
                max_index = i;
            }
        }

        let confidence = ((max_value as f32) - (OUTPUT_ZERO_POINT as f32)) * OUTPUT_SCALE;

        if max_index < 10 && confidence >= 0.7 {
            info!(
                "DETECTED: {} ({}%)",
                CLASS_NAMES[max_index],
                ((confidence * 100.0) as u32),
            );
        } else {
            info!(
                "(best: {} at {}%)",
                CLASS_NAMES[max_index],
                ((confidence * 100.0) as u32),
            );
        }

        // let mut max_amplitude: i32 = 0;
        // for &raw_sample in filled_buffer.iter() {
        //     let sample = (raw_sample as i32) >> 14;
        //     let amplitude = sample.wrapping_abs();
        //     if amplitude > max_amplitude {
        //         max_amplitude = amplitude;
        //     }
        // }
        // info!("Peak amplitude: {}", max_amplitude);

        // Give the processed buffer back to for the next cycle.
        dma_transfer = next_dma_transfer.write_next(filled_buffer);

        // info!("on!");
        // led_pin.set_high().unwrap();
        // timer.delay_ms(200);
        // info!("off!");
        // led_pin.set_low().unwrap();
        // timer.delay_ms(100);
    }
}

/// Program metadata for `picotool info`.
#[unsafe(link_section = ".bu_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Wake Word Detector"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];