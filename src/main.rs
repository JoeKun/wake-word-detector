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

mod audio;
mod inference;
mod output;

mod detectable_words;
mod hardware_resources;

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use rp235x_hal as hal;

use hardware_resources::HardwareResources;

/// Tell the Boot ROM about our application.
#[unsafe(link_section = ".start_block")]
#[used]
#[cfg(rp2350)]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// Entry point to our bare-metal application.
///
/// The `#[hal::entry]` macro ensures the Cortex-M start-up code
/// calls this function as soon as all global variables
/// and the spinlock are initialized.
#[hal::entry]
fn main() -> ! {
    info!("wake-word-detector start.");

    let hardware_resources = HardwareResources::new();
    let HardwareResources {
        microphone_resources,
        output_controller,
    } = hardware_resources;

    // Activate the output controller.
    output_controller.activate();

    // Start the audio task.
    audio::task::run(microphone_resources);

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