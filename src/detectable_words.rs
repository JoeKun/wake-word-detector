/*
 *  detectable_word.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/22/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use num_enum::TryFromPrimitive;

#[derive(Copy, Clone, Debug, PartialEq, TryFromPrimitive)]
#[repr(usize)]
pub enum DetectableWord {
    Six     = 0,
    Seven   = 1,
    Up      = 2,
    Down    = 3,
    Right   = 4,
    Left    = 5,
    On      = 6,
    Off     = 7,
    Wow     = 8,
    Happy   = 9,
    Unknown = 10,
    Silence = 11,
}

impl DetectableWord {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Unknown => false,
            Self::Silence => false,
            _             => true,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Six       => "six",
            Self::Seven     => "seven",
            Self::Up        => "up",
            Self::Down      => "down",
            Self::Right     => "right",
            Self::Left      => "left",
            Self::On        => "on",
            Self::Off       => "off",
            Self::Wow       => "wow",
            Self::Happy     => "happy",
            Self::Unknown   => "unknown",
            Self::Silence   => "silence",
        }
    }
}