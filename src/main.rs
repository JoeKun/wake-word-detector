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

use defmt::*;
use defmt_rtt as _;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
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

/// Entry point to our bare-metal application.
///
/// The `#[hal::entry]` macro ensures the Cortex-M start-up code
/// calls this function as soon as all global variables
/// and the spinlock are initialized.
///
/// The function configures the rp2040 and rp235x peripherals,
/// then toggles a GPIO pin in an infinite loop.
/// If there's an LED connected to that pin, it will blink.
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

    // Set the pins to their default state.
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Configure GPIO25 as an output.
    let mut led_pin = pins.gpio25.into_push_pull_output();
    loop {
        info!("on!");
        led_pin.set_high().unwrap();
        timer.delay_ms(200);
        info!("off!");
        led_pin.set_low().unwrap();
        timer.delay_ms(100);
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