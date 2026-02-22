/*
 *  inference/tflite_wrapper.h
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/15/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

#ifndef TFLITE_WRAPPER_H
#define TFLITE_WRAPPER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    tflite_status_ok                        = 0,
    tflite_status_error_version_mismatch    = -1,
    tflite_status_error_allocation_failed   = -2,
    tflite_status_error_invoke_failed       = -3,
    tflite_status_error_not_initialized     = -4,
} tflite_status_t;

// Initialize the model, op resolver, and interpreter.
tflite_status_t tflite_model_init(
    const uint8_t* model_data,
    uint8_t* tensor_arena,
    size_t tensor_arena_size
);

// Returns the number of bytes used in the tensor arena.
size_t tflite_model_arena_used(void);

// Returns a pointer to the input tensor’s data buffer.
// Caller copies int8 features here before calling invoke.
int8_t* tflite_model_input_data(void);

// Returns the input tensor’s size in bytes.
size_t tflite_model_input_size(void);

// Run inference. Returns `tflite_status_ok` on success.
tflite_status_t tflite_model_invoke(void);

// Returns a pointer to the output tensor’s data buffer
// (12 int8 values).
const int8_t* tflite_model_output_data(void);

// Returns the output tensor’s size in bytes.
size_t tflite_model_output_size(void);

#ifdef __cplusplus
}
#endif

#endif
