/*
 *  output/sequence.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/22/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

use crate::detectable_words::DetectableWord;

use super::instruction::OutputInstruction;

#[derive(Copy, Clone)]
pub struct OutputSequence {
    pub detected_word: DetectableWord,
    instructions: [Option<OutputInstruction>; 20],
    instructions_cursor: usize,
    instructions_count: usize,
    final_cleanup_instructions: [Option<OutputInstruction>; 6],
    final_cleanup_instructions_cursor: usize,
    final_cleanup_instructions_count: usize,
}

impl OutputSequence {
    pub fn new(
        detected_word: DetectableWord,
        previous_output_sequence: Option<Self>
    ) -> Self {
        let mut output_sequence = Self {
            detected_word,
            instructions: [None; 20],
            instructions_cursor: 0,
            instructions_count: 0,
            final_cleanup_instructions: [None; 6],
            final_cleanup_instructions_cursor: 0,
            final_cleanup_instructions_count: 0,
        };
        if let Some(previous_output_sequence) = previous_output_sequence {
            let mut i = previous_output_sequence.final_cleanup_instructions_cursor;
            while i < previous_output_sequence.final_cleanup_instructions_count {
                if let Some(final_cleanup_instruction) = previous_output_sequence.final_cleanup_instructions[i] {
                    output_sequence.enqueue_instruction(final_cleanup_instruction);
                    i += 1;
                }
            }
        }
        output_sequence
    }

    pub fn enqueue_instruction(&mut self, instruction: OutputInstruction) {
        self.instructions[self.instructions_count] = Some(instruction);
        self.instructions_count += 1;
    }

    pub fn enqueue_final_cleanup_instruction(&mut self, final_cleanup_instruction: OutputInstruction) {
        self.final_cleanup_instructions[self.final_cleanup_instructions_count] = Some(final_cleanup_instruction);
        self.final_cleanup_instructions_count += 1;
    }

    pub fn process<F: FnMut(OutputInstruction) -> bool>(
        &mut self,
        mut instruction_handler: F,
    ) -> bool {
        let mut did_process_all_instructions = true;
        let mut i = self.instructions_cursor;
        while i < self.instructions_count {
            if let Some(instruction) = self.instructions[i] {
                did_process_all_instructions = instruction_handler(instruction);
                if did_process_all_instructions {
                    self.instructions_cursor += 1;
                    i += 1;
                } else {
                    break;
                }
            }
        }
        if did_process_all_instructions {
            i = self.final_cleanup_instructions_cursor;
            while i < self.final_cleanup_instructions_count {
                if let Some(final_cleanup_instruction) = self.final_cleanup_instructions[i] {
                    did_process_all_instructions = instruction_handler(final_cleanup_instruction);
                    if did_process_all_instructions {
                        self.final_cleanup_instructions_cursor += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        did_process_all_instructions
    }
}
