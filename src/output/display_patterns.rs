/*
 *  output/display_patterns.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/22/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use super::led_matrix::{
    DisplayPattern,
    LEDMatrixDisplayable,
};

use crate::detectable_words::DetectableWord;

impl LEDMatrixDisplayable for DetectableWord {
    fn as_display_pattern(&self) -> &'static DisplayPattern {
        match self {
            Self::Six       => &SIX_PATTERN,
            Self::Seven     => &SEVEN_PATTERN,
            Self::Up        => &UP_PATTERN,
            Self::Down      => &DOWN_PATTERN,
            Self::Right     => &RIGHT_PATTERN,
            Self::Left      => &LEFT_PATTERN,
            Self::On        => &ON_PATTERN,
            Self::Off       => &OFF_PATTERN,
            Self::Wow       => &WOW_PATTERN,
            Self::Happy     => &HAPPY_PATTERN,
            Self::Unknown   => &UNKNOWN_PATTERN,
            Self::Silence   => &SILENCE_PATTERN,
        }
    }
}

pub const BLANK_PATTERN: DisplayPattern = [[false; 8]; 8];

const SIX_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, false, true,  true,  true,  false, false, false],
    [false, true,  false, false, false, false, false, false],
    [false, true,  true,  true,  false, false, false, false],
    [false, true,  false, false, true,  false, false, false],
    [false, true,  false, false, true,  false, false, false],
    [false, false, true,  true,  false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const SEVEN_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, true,  true,  true,  true,  true,  false, false],
    [false, false, false, false, false, true,  false, false],
    [false, false, false, false, true,  false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const UP_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, true,  true,  true,  false, false, false],
    [false, true,  false, true,  false, true,  false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const DOWN_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, true,  false, true,  false, true,  false, false],
    [false, false, true,  true,  true,  false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const RIGHT_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, true,  false, false, false],
    [false, false, false, false, false, true,  false, false],
    [false, true,  true,  true,  true,  true,  true,  false],
    [false, false, false, false, false, true,  false, false],
    [false, false, false, false, true,  false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const LEFT_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, true,  false, false, false, false, false],
    [false, true,  true,  true,  true,  true,  true,  false],
    [false, false, true,  false, false, false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const ON_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, true ],
    [false, false, false, false, false, false, true,  false],
    [false, true,  false, false, false, true,  false, false],
    [false, false, true,  false, true,  false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const OFF_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, true,  false, false, false, true,  false, false],
    [false, false, true,  false, true,  false, false, false],
    [false, false, false, true,  false, false, false, false],
    [false, false, true,  false, true,  false, false, false],
    [false, true,  false, false, false, true,  false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
];

const WOW_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, true,  true,  false, false, true,  true,  false],
    [false, true,  true,  true,  true,  true,  true,  false],
    [false, true,  true,  true,  true,  true,  true,  false],
    [false, false, true,  true,  true,  true,  false, false],
    [false, false, false, true,  true,  false, false, false],
    [false, false, false, false, false, false, false, false],
];

pub const WOW_PULSING_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],
    [false, true,  true,  false, false, true,  true,  false],
    [true,  true,  true,  true,  true,  true,  true,  true ],
    [true,  true,  true,  true,  true,  true,  true,  true ],
    [false, true,  true,  true,  true,  true,  true,  false],
    [false, false, true,  true,  true,  true,  false, false],
    [false, false, false, true,  true,  false, false, false],
    [false, false, false, false, false, false, false, false],
];

const HAPPY_PATTERN: DisplayPattern = [
    [false, false, true,  true,  true,  true,  false, false],
    [false, true,  false, false, false, false, true,  false],
    [true,  false, true,  false, false, true,  false, true ],
    [true,  false, false, false, false, false, false, true ],
    [true,  false, true,  false, false, true,  false, true ],
    [true,  false, false, true,  true,  false, false, true ],
    [false, true,  false, false, false, false, true,  false],
    [false, false, true,  true,  true,  true,  false, false],
];

const UNKNOWN_PATTERN: DisplayPattern = [
    [false, false, false, false, false, false, false, false],                                                                                                                                                                                         
    [false, false, true,  true,  true,  false, false, false],                                                                                                                                                                                         
    [false, true,  false, false, false, true,  false, false],
    [false, false, false, false, false, true,  false, false],                                                                                                                                                                                         
    [false, false, false, true,  true,  false, false, false],                                                                                                                                                                                         
    [false, false, false, true,  false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, true,  false, false, false, false],
];

const SILENCE_PATTERN: DisplayPattern = [                                                                                                                                                                                                             
    [false, false, false, false, false, false, false, false],                                                                                                                                                                                         
    [false, false, false, false, false, false, false, false],                                                                                                                                                                                         
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],                                                                                                                                                                                         
    [false, true,  false, true,  false, true,  false, false],                                                                                                                                                                                         
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
    [false, false, false, false, false, false, false, false],
];