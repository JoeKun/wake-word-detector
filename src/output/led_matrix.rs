/*
 *  output/led_matrix.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/21/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use embedded_hal::digital::OutputPin;

use super::display_patterns::BLANK_PATTERN;

use crate::hardware_resources::DynOutputPin;

/// Type alias for a display pattern corresponding to the 8x8 LED matrix.
pub type DisplayPattern = [[bool; 8]; 8];

/// Trait for types that can be displayed in an LED matrix.
pub trait LEDMatrixDisplayable {
    fn as_display_pattern(&self) -> &'static DisplayPattern;
}

/// A structure that encapsulates the resources and logic
/// for an 8x8 LED matrix.
pub struct LEDMatrix {
    pub row_pins: [DynOutputPin; 8],
    pub column_pins: [DynOutputPin; 8],

    /// Shared display buffer: an 8x8 boolean grid.
    /// Row 0 is the top row. Column 0 is the leftmost column.
    /// true = LED on, false = LED off.
    display_pattern: DisplayPattern,

    /// The display pattern being rendered currently.
    /// This copy of the display pattern is only updated
    /// at the beginning of a render loop cycle,
    /// when the row index goes back to 0.
    rendering_display_pattern: DisplayPattern,

    /// The index of the current row being scanned.
    current_row_index: usize,

}

impl LEDMatrix {

    // Initialization

    /// Create an LED matrix with row and column pins.
    pub fn new(
        row_pins: [DynOutputPin; 8],
        column_pins: [DynOutputPin; 8],
    ) -> Self {
        Self {
            row_pins,
            column_pins,
            display_pattern: BLANK_PATTERN,
            rendering_display_pattern: BLANK_PATTERN,
            current_row_index: 0,
        }
    }


    // Public methods

    /// Update the shared display buffer from an 8x8 boolean grid.
    /// Row 0 is the top row. Column 0 is the leftmost column.
    /// true = LED on, false = LED off.
    pub fn display(&mut self, display_pattern: &DisplayPattern) {
        self.display_pattern = *display_pattern;
    }

    /// Clear the display buffer.
    pub fn clear_display(&mut self) {
        self.display(&BLANK_PATTERN);
    }

    /// Update state of LED matrix.
    /// This method is meant to be called once every 1ms
    /// to achieve an overall refresh rate of 125Hz.
    pub fn update(&mut self) {
        let row_index = self.current_row_index;
        if row_index == 0 {
            self.rendering_display_pattern = self.display_pattern;
        }
        let display_pattern = self.rendering_display_pattern;

        // Deactivate all rows first to prevent ghosting.
        for row_pin in self.row_pins.iter_mut() {
            row_pin.set_low();
        }

        // Set each column: LOW = LED on, HIGH = LED off.
        for column_index in 0..8_usize {
            if display_pattern[row_index][column_index] {
                self.column_pins[column_index].set_low();
            } else {
                self.column_pins[column_index].set_high();
            }
        }

        // Activate the current row.
        self.row_pins[row_index].set_high();

        // Advance to the next row.
        self.current_row_index = (row_index + 1) % 8;

    }
    
}
