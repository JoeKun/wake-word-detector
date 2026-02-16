#
#  convert_to_tflite.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import tensorflow as tf
from tensorflow import keras
import numpy as np
from pathlib import Path
import shutil

from data_loader import SpeechDataset


def create_representative_dataset(num_samples: int = 200):
    """Create a representative dataset generator for quantization calibration."""
    print(f"Loading {num_samples} samples for quantization calibration...")

    dataset = SpeechDataset(
        data_directory_path=Path("data/speech_commands"),
        target_words=[
            "six",
            "seven",
            "up",
            "down",
            "right",
            "left",
            "on",
            "off",
            "wow",
            "happy",
        ],
    )

    # Get samples from the dataset (batch_size=1 for individual samples).
    tf_dataset = dataset.get_dataset(batch_size=1, shuffle=True)

    # Collect samples into a list.
    samples = []
    for mfcc, _ in tf_dataset.take(num_samples):
        samples.append(mfcc.numpy().astype(np.float32))

    print(f"Collected {len(samples)} samples")
    if samples:
        print(f"Sample shape: {samples[0].shape}, dtype: {samples[0].dtype}")
        print(f"Sample value range: [{samples[0].min():.3f}, {samples[0].max():.3f}]")

    def representative_dataset():
        for sample in samples:
            yield [sample]

    return representative_dataset


def convert_to_tflite(
    model_path: str,
    output_path: str = "models/model_quantized.tflite",
) -> str:
    """Convert Keras model to TFLite with full int8 quantization."""
    model = keras.models.load_model(model_path)

    print("\nModel architecture:")
    model.summary()

    print("\nConverting to TFLite with full int8 quantization...")

    # Save to SavedModel first, then convert (sometimes more reliable).
    saved_model_dir = Path("models/temp_saved_model")
    model.export(saved_model_dir)
    converter = tf.lite.TFLiteConverter.from_saved_model(str(saved_model_dir))

    converter.optimizations = [tf.lite.Optimize.DEFAULT]
    converter.representative_dataset = create_representative_dataset()
    converter.target_spec.supported_ops = [tf.lite.OpsSet.TFLITE_BUILTINS_INT8]
    converter.inference_input_type = tf.int8
    converter.inference_output_type = tf.int8

    tflite_model = converter.convert()

    shutil.rmtree(saved_model_dir)

    print(f"Saving model to {output_path}...")
    with open(output_path, "wb") as f:
        f.write(tflite_model)

    print(f"\n{'=' * 60}")
    print(f"Model saved to: {output_path}")
    print(f"Model size: {len(tflite_model) / 1024:.1f} KB")
    print(f"{'=' * 60}")

    print("\nVerifying model...")
    interpreter = tf.lite.Interpreter(model_path=output_path)
    interpreter.allocate_tensors()

    input_details = interpreter.get_input_details()
    output_details = interpreter.get_output_details()

    print(f"✓ Input shape: {input_details[0]['shape']}")
    print(f"✓ Input dtype: {input_details[0]['dtype']}")
    print(f"✓ Output shape: {output_details[0]['shape']}")
    print(f"✓ Output dtype: {output_details[0]['dtype']}")

    input_scale = input_details[0]["quantization"][0]
    input_zero_point = input_details[0]["quantization"][1]
    output_scale = output_details[0]["quantization"][0]
    output_zero_point = output_details[0]["quantization"][1]
    print(f"✓ Input quantization: scale={input_scale}, zero_point={input_zero_point}")
    print(f"✓ Output quantization: scale={output_scale}, zero_point={output_zero_point}")

    # Inspect all tensors to see what's quantized.
    print("\n" + "=" * 60)
    print("Tensor inspection (checking quantization):")
    print("=" * 60)
    tensor_details = interpreter.get_tensor_details()
    
    float_tensors = []
    int_tensors = []
    for tensor in tensor_details:
        dtype_str = str(tensor["dtype"])
        if "float" in dtype_str:
            float_tensors.append(tensor["name"])
        else:
            int_tensors.append((tensor["name"], tensor["dtype"]))
    
    print(f"\nQuantized tensors (int8): {len(int_tensors)}")
    print(f"Float tensors: {len(float_tensors)}")
    
    if float_tensors:
        print("\n⚠ Float tensors found:")
        for name in float_tensors[:10]:  # Show first 10
            print(f"  - {name}")
        if len(float_tensors) > 10:
            print(f"  ... and {len(float_tensors) - 10} more")

    print("\n✓ Model verified and ready for deployment")

    return output_path


if __name__ == "__main__":
    convert_to_tflite("models/best_model_fused.keras")