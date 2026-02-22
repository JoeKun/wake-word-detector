/*
 *  output/controller.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/22/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use core::cell::RefCell;
use cortex_m::peripheral::NVIC;
use critical_section::Mutex;
use defmt::*;
use embedded_hal::digital::OutputPin;
use rp235x_hal as hal;
use hal::fugit::{
    ExtU32,
    ExtU64,
    Instant,
};
use hal::gpio::{
    bank0::Gpio25,
    FunctionSio,
    SioOutput,
    PullDown,
    Pin,
};
use hal::pac::Interrupt;
use hal::timer::Alarm;

use crate::detectable_words::DetectableWord;

use super::instruction::OutputInstruction;
use super::sequence::OutputSequence;

use super::display_patterns::{
    WOW_PULSING_PATTERN,
};
use super::led_matrix::{
    LEDMatrix,
    LEDMatrixDisplayable,
};


/// A static variable to hold the output controller instance
/// to be able to retrieve it in the interrupt handler below.
static OUTPUT_CONTROLLER: Mutex<RefCell<Option<OutputController>>> = 
    Mutex::new(RefCell::new(None));

/// A structure that encapsulates the resources and logic
/// for all the outputs of the wake word detector.
pub struct OutputController {
    pub led_matrix: LEDMatrix,
    pub status_led_pin: Pin<Gpio25, FunctionSio<SioOutput>, PullDown>,

    /// The timer used to keep track of time.
    pub timer: hal::Timer<hal::timer::CopyableTimer0>,

    /// The timer alarm fires every 1ms, allowing the output
    /// controller to update the state of the hardware resources
    /// to match the desired output.
    interrupt_alarm: hal::timer::Alarm0<hal::timer::CopyableTimer0>,

    /// A boolean that indicates if the wake word detector is
    /// looking turned on.
    /// When the wake word detector is turned off,
    /// it only responds to the word "On".
    is_on: bool,

    /// The last detected word.
    last_detected_word: DetectableWord,

    /// The timestamp when the last detected word was recognized.
    last_detected_timestamp: Instant<u64, 1, 1_000_000>,

    /// The current output sequence.
    current_output_sequence: Option<OutputSequence>,

}

/// Update outputs for the detected word.
/// The confidence parameter is expected to be a floating-point
/// value between 0.0 and 1.0.
pub fn handle_detected_word(
    detected_word: DetectableWord, 
    confidence: f32,
) {
    critical_section::with(|critical_section| {
        if let Some(output_controller) = OUTPUT_CONTROLLER.borrow_ref_mut(critical_section).as_mut() {
            output_controller.handle_detected_word(
                detected_word, 
                confidence,
            );
        }
    });
}


impl OutputController {

    // Initialization

    pub fn new(
        led_matrix: LEDMatrix,
        status_led_pin: Pin<Gpio25, FunctionSio<SioOutput>, PullDown>,
        mut timer: hal::Timer<hal::timer::CopyableTimer0>,
    ) -> Self {
        Self {
            led_matrix,
            status_led_pin,
            timer,
            interrupt_alarm: timer.alarm_0()
                .expect("Unable to make an alarm from provided timer for the output controller."),
            is_on: true,
            last_detected_word: DetectableWord::Unknown,
            last_detected_timestamp: timer.get_counter(),
            current_output_sequence: None,
        }
    }


    // Word detection logic

    fn handle_detected_word(
        &mut self,
        detected_word: DetectableWord, 
        confidence: f32,
    ) {
        let mut adaptive_confidence_threshold: f32 = 0.7;
        if !self.is_on && detected_word != DetectableWord::On {
            return;
        }

        let current_timestamp = self.timer.get_counter();
        if (current_timestamp - self.last_detected_timestamp).to_millis() > 2000 {
            self.last_detected_word = DetectableWord::Unknown;
        } else {
            if self.last_detected_word == DetectableWord::Six {
                adaptive_confidence_threshold = 0.45;
            }
        }

        if detected_word.is_valid() && confidence >= adaptive_confidence_threshold {
            let mut is_repeated_detection = false;
            let previous_output_sequence = self.current_output_sequence.take();
            if let Some(previous_output_sequence) = previous_output_sequence {
                if previous_output_sequence.detected_word == detected_word {
                    is_repeated_detection = true;
                }
            }
            info!(
                "DETECTED: {} ({}%{})",
                detected_word.as_str(),
                ((confidence * 100.0) as u32),
                (if is_repeated_detection { " - repeated" } else { "" }),
            );

            if is_repeated_detection {
                self.current_output_sequence = previous_output_sequence;
            } else {
                let mut output_sequence = OutputSequence::new(
                    detected_word, 
                    previous_output_sequence,
                );
                if !self.is_on && detected_word == DetectableWord::On {
                    self.is_on = true;
                    output_sequence.enqueue_instruction(OutputInstruction::TurnOnStatusLED);
                } else if self.is_on && detected_word == DetectableWord::Off {
                    self.is_on = false;
                    output_sequence.enqueue_instruction(OutputInstruction::TurnOffStatusLED);
                }

                output_sequence.enqueue_instruction(
                    OutputInstruction::DisplayPattern(detected_word.as_display_pattern())
                );

                match (self.last_detected_word, detected_word) {
                    (DetectableWord::Six, DetectableWord::Seven) => {
                        let happy = DetectableWord::Happy;
                        let mut future_timestamp = current_timestamp;
                        for i in 0..3 {
                            future_timestamp = future_timestamp + 500_000_u64.micros();
                            output_sequence.enqueue_instruction(
                                OutputInstruction::Wait(future_timestamp)
                            );
                            output_sequence.enqueue_instruction(
                                OutputInstruction::DisplayPattern(happy.as_display_pattern())
                            );
                            future_timestamp = future_timestamp + 500_000_u64.micros();
                            output_sequence.enqueue_instruction(
                                OutputInstruction::Wait(future_timestamp)
                            );
                            if i < 2 {
                                output_sequence.enqueue_instruction(OutputInstruction::ClearDisplay);
                            }
                        }
                    },
                    (_, DetectableWord::Wow) => {
                        let mut future_timestamp = current_timestamp;
                        for _ in 0..3 {
                            future_timestamp = future_timestamp + 500_000_u64.micros();
                            output_sequence.enqueue_instruction(
                                OutputInstruction::Wait(future_timestamp)
                            );
                            output_sequence.enqueue_instruction(
                                OutputInstruction::DisplayPattern(&WOW_PULSING_PATTERN)
                            );
                            future_timestamp = future_timestamp + 500_000_u64.micros();
                            output_sequence.enqueue_instruction(
                                OutputInstruction::Wait(future_timestamp)
                            );
                            output_sequence.enqueue_instruction(
                                OutputInstruction::DisplayPattern(detected_word.as_display_pattern())
                            );
                        }
                        future_timestamp = future_timestamp + 500_000_u64.micros();
                        output_sequence.enqueue_instruction(
                            OutputInstruction::Wait(future_timestamp)
                        );
                    },
                    _ => {
                        output_sequence.enqueue_instruction(
                            OutputInstruction::Wait(current_timestamp + 2_000_000_u64.micros())
                        );
                    },
                }
                output_sequence.enqueue_final_cleanup_instruction(OutputInstruction::ClearDisplay);
                self.current_output_sequence = Some(output_sequence);
            }

            self.last_detected_word = detected_word;
            self.last_detected_timestamp = current_timestamp;
        } else {
            info!(
                "(best: {} at {}%)",
                detected_word.as_str(),
                ((confidence * 100.0) as u32),
            );
        }
    }


    // Interrupt handling

    /// Initializes interrupt-driven output.
    /// Call once from main before starting other work for the main loop.
    pub fn activate(mut self) {

        // Turn on the status LED upon activation.
        if self.is_on {
            self.status_led_pin.set_high().ok();
        }

        // Configure interrupt alarm.
        self.interrupt_alarm.schedule(1_000_u32.micros()).ok();
        self.interrupt_alarm.enable_interrupt();

        critical_section::with(|critical_section| {
            *OUTPUT_CONTROLLER.borrow_ref_mut(critical_section) = Some(self);
        });

        // Enable the interrupt handler `output_controller_interrupt`, 
        // which is exported as `TIMER0_IRQ_0`, by unmasking that interrupt 
        // using the Nested Vectored Interrupt Controller (NVIC).
        unsafe {
            NVIC::unmask(Interrupt::TIMER0_IRQ_0);
        }

    }

    fn handle_interrupt(&mut self) {
        self.interrupt_alarm.clear_interrupt();
        self.interrupt_alarm.schedule(1_000_u32.micros()).ok();
        self.update();
    }

    fn update(&mut self) {

        // Process current output sequence.
        if let Some(mut current_output_sequence) = self.current_output_sequence.take() {
            let did_process_all_instructions = current_output_sequence.process(
                |instruction| -> bool {
                    self.process(instruction)
                }
            );
            if !did_process_all_instructions {
                self.current_output_sequence = Some(current_output_sequence);
            }
        }

        // Update the LED matrix.
        self.led_matrix.update();

    }

    fn process(
        &mut self, 
        instruction: OutputInstruction,
    ) -> bool {
        let did_process_instruction: bool;
        match instruction {
            OutputInstruction::DisplayPattern(display_pattern) => {
                self.led_matrix.display(display_pattern);
                did_process_instruction = true;
            },
            OutputInstruction::TurnOnStatusLED => {
                self.status_led_pin.set_high().ok();
                did_process_instruction = true;
            },
            OutputInstruction::TurnOffStatusLED => {
                self.status_led_pin.set_low().ok();
                did_process_instruction = true;
            },
            OutputInstruction::Wait(delay_expiration_timestamp) => {
                let current_timestamp = self.timer.get_counter();
                if current_timestamp > delay_expiration_timestamp {
                    did_process_instruction = true;
                } else {
                    did_process_instruction = false;
                }
            },
            OutputInstruction::ClearDisplay => {
                self.led_matrix.clear_display();
                did_process_instruction = true;
            },
        }
        did_process_instruction
    }

}


// Global interrupt handler function

/// Interrupt handler: fires every 1ms, activates the next LED matrix row.
#[unsafe(export_name = "TIMER0_IRQ_0")]
unsafe extern "C" fn output_controller_interrupt() {
    critical_section::with(|critical_section| {
        let mut output_controller_ref = 
            OUTPUT_CONTROLLER.borrow_ref_mut(critical_section);
        if let Some(output_controller) = output_controller_ref.as_mut() {
            output_controller.handle_interrupt();
        }
    });
}
