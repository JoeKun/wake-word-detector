/*
 *  tflite-wrapper/tflite_wrapper.cc
 *  wake-word-detector
 *
 *  Created by Joel Lopes Da Silva on 2/15/26.
 *  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
 *
 */

#include "tflite_wrapper.h"

#include <tensorflow/lite/micro/micro_interpreter.h>
#include <tensorflow/lite/micro/micro_mutable_op_resolver.h>
#include <tensorflow/lite/micro/cortex_m_generic/debug_log_callback.h> 
#include <tensorflow/lite/schema/schema_generated.h>

#include <new>

extern "C" void tflite_debug_log_string(const char* s);

// Static storage - no heap allocation needed.
static const tflite::Model* model = nullptr;

alignas(tflite::MicroMutableOpResolver<5>)
    static uint8_t resolver_buffer[sizeof(tflite::MicroMutableOpResolver<5>)];
static tflite::MicroMutableOpResolver<5>* resolver = nullptr;

alignas(tflite::MicroInterpreter)
    static uint8_t interpreter_buffer[sizeof(tflite::MicroInterpreter)];
static tflite::MicroInterpreter* interpreter = nullptr;

extern "C"
tflite_status_t tflite_model_init(
    const uint8_t* model_data,
    uint8_t* tensor_arena,
    size_t tensor_arena_size
) {
    // Override TFLite Micro’s `DebugLog` function with our own version
    // that routes the log to Rust and then to `defmt`.
    RegisterDebugLogCallback(tflite_debug_log_string);

    model = tflite::GetModel(model_data);
    if (model->version() != TFLITE_SCHEMA_VERSION) {
        return tflite_status_error_version_mismatch;
    }

    resolver = new (resolver_buffer) tflite::MicroMutableOpResolver<5>();
    resolver->AddConv2D();
    resolver->AddMaxPool2D();
    resolver->AddMean();                // Needed for GlobalAveragePooling2D.
    resolver->AddFullyConnected();
    resolver->AddSoftmax();

    interpreter = new (interpreter_buffer) tflite::MicroInterpreter(
        model, 
        *resolver, 
        tensor_arena, 
        tensor_arena_size
    );

    if (interpreter->AllocateTensors() != kTfLiteOk) {
        return tflite_status_error_allocation_failed;
    }

    return tflite_status_ok;
}

extern "C"
size_t tflite_model_arena_used(void) {
    if (interpreter == nullptr) return 0;
    return interpreter->arena_used_bytes();
}

extern "C"
int8_t* tflite_model_input_data(void) {
    if (interpreter == nullptr) return nullptr;
    return interpreter->input(0)->data.int8;
}

extern "C"
size_t tflite_model_input_size(void) {
    if (interpreter == nullptr) return 0;
    return interpreter->input(0)->bytes;
}

extern "C"
tflite_status_t tflite_model_invoke(void) {
    if (interpreter == nullptr) return tflite_status_error_not_initialized;
    if (interpreter->Invoke() != kTfLiteOk) {
        return tflite_status_error_invoke_failed;
    }
    return tflite_status_ok;
}

extern "C"
const int8_t* tflite_model_output_data(void) {
    if (interpreter == nullptr) return nullptr;
    return interpreter->output(0)->data.int8;
}

extern "C"
size_t tflite_model_output_size(void) {
    if (interpreter == nullptr) return 0;
    return interpreter->output(0)->bytes;
}

