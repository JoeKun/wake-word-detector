/*
 *  hardware_resources.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/22/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use rp235x_hal as hal;

use hal::gpio::{
    DynPinId,
    FunctionPio0,
    FunctionSioOutput,
    PullDown,
    Pin,
};

use hal::dma::DMAExt;
use hal::pio::PIOExt;

use crate::audio::task::MicrophoneResources;
use crate::output::controller::OutputController;
use crate::output::led_matrix::LEDMatrix;

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Type alias for a type-erased GPIO output pin.
pub type DynOutputPin = Pin<DynPinId, FunctionSioOutput, PullDown>;

/// Structure for a GPIO function PIO pin with its pin index.
pub struct FunctionPioPin {
    pub index: u8,
    _underlying_pin: Pin<DynPinId, FunctionPio0, PullDown>
}

pub struct HardwareResources {
    pub microphone_resources: MicrophoneResources,
    pub output_controller: OutputController,
}

impl HardwareResources {
    pub fn new() -> Self {

        // Grab our peripheral access crate.
        let mut pac = hal::pac::Peripherals::take()
            .expect("Unable to grab the peripheral access crate.");

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
        .expect("Unable to initialize clocks");

        // The single-cycle I/O block controls our GPIO pins.
        let sio = hal::Sio::new(pac.SIO);

        // Set the pins to their default state.
        let pins = hal::gpio::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        // Create a timer for the microphone resources.
        let microphone_timer = hal::Timer::new_timer1(
            pac.TIMER1,
            &mut pac.RESETS,
            &clocks,
        );

        // Split PIO0 into the controller and 4 state machines.
        // We only need one of the state machines.
        let (
            microphone_pio, 
            unitialized_microphone_pio_state_machine, 
            ..
        ) = pac.PIO0.split(&mut pac.RESETS);

        // Split the DMA peripheral.
        let dma_channels = pac.DMA.split(&mut pac.RESETS);

        // Set microphone pins to PIO function.
        let microphone_resources = MicrophoneResources {
            // BCLK = GP13
            bclk_pin: FunctionPioPin {
                index: 13,
                _underlying_pin: pins.gpio13.into_function::<FunctionPio0>().into_dyn_pin(),
            },
            // LRCL = GP14 (must be consecutive to BCLK)
            lrcl_pin: FunctionPioPin {
                index: 14,
                _underlying_pin: pins.gpio14.into_function::<FunctionPio0>().into_dyn_pin(),
            },
            // DOUT = GP15
            dout_pin: FunctionPioPin {
                index: 15,
                _underlying_pin: pins.gpio15.into_function::<FunctionPio0>().into_dyn_pin(),
            },
            timer: microphone_timer,
            pio: microphone_pio,
            unitialized_state_machine: unitialized_microphone_pio_state_machine,
            dma_channels: dma_channels,
        };

        // Create the LED matrix.
        let led_matrix = LEDMatrix::new(

            // Row pins (anodes, left side of Pico).
            [
                pins.gpio5.into_push_pull_output().into_dyn_pin(),
                pins.gpio6.into_push_pull_output().into_dyn_pin(),
                pins.gpio7.into_push_pull_output().into_dyn_pin(),
                pins.gpio8.into_push_pull_output().into_dyn_pin(),
                pins.gpio9.into_push_pull_output().into_dyn_pin(),
                pins.gpio10.into_push_pull_output().into_dyn_pin(),
                pins.gpio11.into_push_pull_output().into_dyn_pin(),
                pins.gpio12.into_push_pull_output().into_dyn_pin(),
            ],

            // Column pins (cathodes, right side of Pico).
            [
                pins.gpio16.into_push_pull_output().into_dyn_pin(),
                pins.gpio17.into_push_pull_output().into_dyn_pin(),
                pins.gpio18.into_push_pull_output().into_dyn_pin(),
                pins.gpio19.into_push_pull_output().into_dyn_pin(),
                pins.gpio20.into_push_pull_output().into_dyn_pin(),
                pins.gpio21.into_push_pull_output().into_dyn_pin(),
                pins.gpio22.into_push_pull_output().into_dyn_pin(),
                pins.gpio26.into_push_pull_output().into_dyn_pin(),
            ],

        );

        // Create a timer for the output controller.
        let output_controller_timer = hal::Timer::new_timer0(
            pac.TIMER0, 
            &mut pac.RESETS, 
            &clocks,
        );

        // Create the output controller.
        let output_controller = OutputController::new(
            led_matrix, 
            pins.gpio25.into_push_pull_output(), 
            output_controller_timer,
        );

        Self {
            microphone_resources,
            output_controller,
        }
    }
}