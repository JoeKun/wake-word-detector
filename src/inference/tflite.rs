/*
 *  tflite.rs
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/14/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

#[unsafe(no_mangle)]
pub extern "C" fn tflite_debug_log_string(s: *const core::ffi::c_char) {
    let c_str = unsafe { core::ffi::CStr::from_ptr(s) };
    if let Ok(text) = c_str.to_str() {
        defmt::info!("TFLite DebugLog: {}", text);
    }
}

#[repr(i32)]
#[derive(Debug, PartialEq, defmt::Format)]
#[allow(dead_code)]
pub enum Status {
    Ok                      = 0,
    ErrorVersionMismatch    = -1,
    ErrorAllocationFailed   = -2,
    ErrorInvokeFailed       = -3,
    ErrorNotInitialized     = -4,
}

unsafe extern "C" {
    pub fn tflite_model_init(
        model_data: *const u8,
        tensor_arena: *mut u8,
        tensor_arena_size: usize,
    ) -> Status;

    pub fn tflite_model_arena_used() -> usize;

    pub fn tflite_model_input_data() -> *mut i8;

    pub fn tflite_model_input_size() -> usize;

    pub fn tflite_model_invoke() -> Status;

    pub fn tflite_model_output_data() -> *const i8;

    pub fn tflite_model_output_size() -> usize;

    pub fn tflite_model_input_scale() -> f32;

    pub fn tflite_model_input_zero_point() -> i32;

    pub fn tflite_model_output_scale() -> f32;

    pub fn tflite_model_output_zero_point() -> i32;
}