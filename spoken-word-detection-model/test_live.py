#
#  test_live.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import tensorflow as tf
from tensorflow import keras
import numpy as np
import sounddevice as sd
from data_loader import SpeechDataset
from pathlib import Path
import argparse

def record_audio(duration: float = 1.0, sample_rate: int = 16000) -> np.ndarray:
    """Record audio from the microphone."""
    print(f"Recording for {duration} second(s)... Speak now!")
    audio = sd.rec(
        int(duration * sample_rate), 
        samplerate=sample_rate, 
        channels=1, 
        dtype="int16"
    )
    # Wait until recording is finished.
    sd.wait()
    print("Recording complete!")
    return audio.flatten()

def predict_word_keras(
        model_path: str, 
        audio_samples: np.ndarray, 
        dataset: SpeechDataset
    ) -> tuple[str, float]:
    """
    Predict the word from audio samples using Keras model.
    
    Returns:
        (predicted_word, confidence)
    """
    # Load model (cached after first call).
    if not hasattr(predict_word_keras, "model"):
        predict_word_keras.model = keras.models.load_model(model_path)
    
    model = predict_word_keras.model
    
    # Convert to TensorFlow tensor.
    audio_tensor = tf.convert_to_tensor(audio_samples, dtype=tf.float32)
    
    # Normalize from int16 to float32 [-1, 1].
    audio_tensor = audio_tensor / 32768.0
    
    # Extract MFCC features (reusing the dataset’s method).
    mfcc = dataset.extract_mfcc(audio_tensor)
    
    # Add batch and channel dimensions.
    mfcc = tf.expand_dims(mfcc, 0)  # Add batch dimension.
    mfcc = tf.expand_dims(mfcc, -1)  # Add channel dimension.
    
    # Run inference.
    predictions = model.predict(mfcc, verbose=0)
    
    # Get the predicted class and confidence.
    predicted_index = np.argmax(predictions[0])
    confidence = predictions[0][predicted_index]
    predicted_word = dataset.classes[predicted_index]
    
    return predicted_word, confidence

def predict_word_tflite(
        model_path: str, 
        audio_samples: np.ndarray, 
        dataset: SpeechDataset
    ) -> tuple[str, float]:
    """
    Predict the word from audio samples using TFLite model.
    
    Returns:
        (predicted_word, confidence)
    """
    # Load TFLite model (cached after first call).
    if not hasattr(predict_word_tflite, "interpreter"):
        predict_word_tflite.interpreter = tf.lite.Interpreter(model_path=model_path)
        predict_word_tflite.interpreter.allocate_tensors()
        predict_word_tflite.input_details = predict_word_tflite.interpreter.get_input_details()
        predict_word_tflite.output_details = predict_word_tflite.interpreter.get_output_details()
    
    interpreter = predict_word_tflite.interpreter
    input_details = predict_word_tflite.input_details
    output_details = predict_word_tflite.output_details
    
    # Convert to TensorFlow tensor.
    audio_tensor = tf.convert_to_tensor(audio_samples, dtype=tf.float32)
    
    # Normalize from int16 to float32 [-1, 1].
    audio_tensor = audio_tensor / 32768.0
    
    # Extract MFCC features (reusing the dataset’s method).
    mfcc = dataset.extract_mfcc(audio_tensor)
    
    # Add batch and channel dimensions.
    mfcc = tf.expand_dims(mfcc, 0)  # Add batch dimension.
    mfcc = tf.expand_dims(mfcc, -1)  # Add channel dimension.
    mfcc = mfcc.numpy()  # Convert to numpy.
    
    # Quantize input if the model expects int8.
    input_dtype = input_details[0]["dtype"]
    if input_dtype == np.int8:
        # Get quantization parameters.
        input_scale = input_details[0]["quantization"][0]
        input_zero_point = input_details[0]["quantization"][1]
        
        # Quantize: float_value = (quantized_value - zero_point) * scale.
        # So: quantized_value = float_value / scale + zero_point.
        mfcc_quantized = (mfcc / input_scale + input_zero_point).astype(np.int8)
        input_data = mfcc_quantized
    else:
        input_data = mfcc.astype(np.float32)
    
    # Run inference.
    interpreter.set_tensor(input_details[0]["index"], input_data)
    interpreter.invoke()
    
    # Get output.
    output_data = interpreter.get_tensor(output_details[0]["index"])
    
    # Dequantize output if needed.
    output_dtype = output_details[0]["dtype"]
    if output_dtype == np.int8:
        output_scale = output_details[0]["quantization"][0]
        output_zero_point = output_details[0]["quantization"][1]
        probabilities = (output_data.astype(np.float32) - output_zero_point) * output_scale
    else:
        probabilities = output_data
    
    # Get the predicted class and confidence.
    predicted_index = np.argmax(probabilities[0])
    confidence = probabilities[0][predicted_index]
    predicted_word = dataset.classes[predicted_index]
    
    return predicted_word, confidence

def main():
    # Parse command line arguments.
    parser = argparse.ArgumentParser(description="Test wake word detection with live microphone input")
    parser.add_argument(
        "--quantized",
        action="store_true",
        help="Use quantized TFLite model instead of Keras model"
    )
    args = parser.parse_args()
    
    # Initialize dataset (we just need it for MFCC extraction and class names).
    data_directory_path = Path("data/speech_commands")
    dataset = SpeechDataset(
        data_directory_path,
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
        ]
    )
    
    # Select model and prediction function.
    if args.quantized:
        model_path = "models/model_quantized.tflite"
        model_type = "TFLite Quantized"
        predict_function = predict_word_tflite
    else:
        model_path = "models/best_model.keras"
        model_type = "Keras"
        predict_function = predict_word_keras
    
    # Check if model exists.
    if not Path(model_path).exists():
        print(f"Error: {model_path} not found!")
        if args.quantized:
            print("Please run convert_to_tflite.py first to create the quantized model.")
        else:
            print("Please run train.py first to create the model.")
        return
    
    # Get model size.
    model_size_kb = Path(model_path).stat().st_size / 1024
    
    print("="*60)
    print(f"Wake Word Detector - Live Test ({model_type} Model)")
    print("="*60)
    print(f"Model: {model_path} ({model_size_kb:.1f} KB)")
    print(f"Available words: {', '.join(dataset.classes)}")
    print("="*60)
    
    # Confidence threshold (from evaluation).
    confidence_threshold = 0.7
    
    while True:
        input("\nPress Enter to record a word (or Ctrl+C to quit)...")
        
        # Record audio.
        audio = record_audio(duration=1.0, sample_rate=16000)
        
        # Predict.
        word, confidence = predict_function(model_path, audio, dataset)
        
        # Display result.
        print(f"\n{'='*60}")
        if confidence >= confidence_threshold:
            print(f"✓ Detected: {word.upper()}")
            print(f"  Confidence: {confidence:.2%}")
        else:
            print(f"✗ Low confidence detection: {word}")
            print(f"  Confidence: {confidence:.2%} (threshold: {confidence_threshold:.2%})")
        print(f"{'='*60}")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nTesting complete!")