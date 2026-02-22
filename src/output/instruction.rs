/*
 *  output/instruction.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/22/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use rp235x_hal as hal;
use hal::fugit::Instant;

use super::led_matrix::DisplayPattern;

#[derive(Copy, Clone)]
pub enum OutputInstruction {
    DisplayPattern(&'static DisplayPattern),
    TurnOnStatusLED,
    TurnOffStatusLED,
    Wait(Instant<u64, 1, 1_000_000>),
    ClearDisplay,
}