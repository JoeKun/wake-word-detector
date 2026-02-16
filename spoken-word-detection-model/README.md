# Wake Word Detector - Model Training

A machine learning project to train a keyword spotting model for embedded deployment on Raspberry Pi Pico.

## Overview

This project trains a convolutional neural network (CNN) to recognize specific wake words from audio input.
The model is designed to run on resource-constrained embedded devices like the Raspberry Pi Pico.

**Target words:** six, seven, up, down, right, left, on, off, wow, happy
**Additional classes:** unknown (other words), silence

## Requirements

### Hardware
- Mac with Apple Silicon (M1/M2/M3) recommended for GPU acceleration
- Microphone for live testing

### Software
- Python 3.11 (required for TensorFlow Metal support)
- Virtual environment

## Setup

### 1. Install Python 3.11

```bash
brew install python@3.11
```

### 2. Create Virtual Environment

```bash
cd /path/to/wake-word-detector/spoken-word-detection-model
python3.11 -m venv .venv
source .venv/bin/activate
```

### 3. Install Dependencies

```bash
# Upgrade pip
pip install --upgrade pip

# Install TensorFlow with Metal GPU support (Apple Silicon only)
pip install tensorflow-macos==2.15.0
pip install tensorflow-metal==1.1.0

# Install other dependencies
pip install numpy scipy librosa matplotlib scikit-learn seaborn sounddevice
```

### 4. Verify GPU Setup

```bash
python -c "import tensorflow as tf; print('TensorFlow version:', tf.__version__); print('Num GPUs:', len(tf.config.list_physical_devices('GPU')))"
```

Expected output: `Num GPUs: 1`

## Project Structure
```
wake-word-detector/
└── spoken-word-detection-model/
    ├── data/
    │   └── speech_commands/                    # Downloaded dataset
    ├── models/
    │   ├── best_model.keras                    # Best model from training
    │   ├── best_model_fused.keras              # BatchNormalization fused into Conv2D
    │   ├── final_model.keras                   # Final model
    │   └── model_quantized.tflite              # Int8 quantized TFLite model
    ├── logs/                                   # TensorBoard logs
    ├── prepare_dataset.py                      # Download and explore dataset
    ├── data_loader.py                          # Dataset loading and preprocessing
    ├── train_test_split.py                     # Train/validation/test splitting
    ├── model.py                                # CNN model architecture
    ├── train.py                                # Training script
    ├── evaluate.py                             # Model evaluation and metrics
    ├── test_live.py                            # Live microphone testing
    ├── fuse_batch_normalization.py             # Fuse BatchNormalization into Conv2D weights
    └── convert_to_tflite.py                    # Convert to TensorFlow Lite
```

## Usage

### Step 1: Download Dataset

```bash
python prepare_dataset.py
```

Downloads Google Speech Commands dataset (~2GB) with 35 word classes and ~105,000 audio samples.

### Step 2: Train Model

```bash
python train.py
```

**Training details:**
- **Dataset split:** 70% train, 15% validation, 15% test
- **Epochs:** Up to 50 with early stopping (best model typically saved around epoch 40-45)
- **Batch size:** 32
- **Optimizer:** Adam (legacy version for M1/M2 compatibility)
- **Learning rate:** 0.001 with reduction on plateau
- **Data augmentation:** None (can be added if needed)

**Training time:**
- With GPU (M1 Max): ~50 seconds per epoch
- Without GPU: ~3-5 minutes per epoch

**Callbacks:**
- Early stopping: Stops if validation loss doesn't improve for 10 epochs
- Model checkpoint: Saves best model based on validation accuracy
- Learning rate reduction: Reduces LR by 50% if validation loss plateaus
- TensorBoard: Logs training metrics

### Step 3: Evaluate Model

```bash
python evaluate.py
```

**Evaluation metrics:**
- Overall test accuracy
- Per-class precision, recall, F1-score
- Confusion matrix (saved as `confusion_matrix.png`)
- Confidence threshold analysis
- False positive rate analysis

**Target performance:**
- ✓ Overall accuracy >85%: Acceptable
- ✓ Overall accuracy >90%: Good
- ✓ Overall accuracy >92%: Excellent

**Current results:**
- Test accuracy: **92.49%**
- Recommended confidence threshold: **0.7**
- False positive rate: **0.94%** (at 0.7 threshold)

### Step 4: Fuse BatchNormalization into Conv2D

```bash
python fuse_batch_normalization.py
```

Fuses the trained BatchNormalization parameters into the preceding Conv2D weights, eliminating BatchNormalization as separate operations. This produces `models/best_model_fused.keras` with identical outputs but fewer layers.

### Step 5: Convert to TensorFlow Lite

```bash
python convert_to_tflite.py
```

Converts the fused Keras model to a fully int8 quantized TensorFlow Lite model for embedded deployment:
- **Input format:** Float32 Keras model (~51 KB)
- **Output format:** Int8 quantized TFLite model (~27 KB)
- **Quantization:** Full int8 (weights and activations) with representative dataset calibration

### Step 6: Test Live Audio

```bash
python test_live.py
```

Interactive testing with your Mac's microphone:
1. Press Enter to start recording
2. Speak a wake word
3. See the prediction and confidence score

Use `--quantized` to test with the TFLite model instead of the Keras model:

```bash
python test_live.py --quantized
```

## Model Architecture

```
Input: (97, 40, 1) - MFCC features from 1-second audio
├── Conv2D(32) + BatchNormalization + ReLU + MaxPool + Dropout(0.25)
├── DepthwiseConv2D(3x3) + Conv2D(64, 1x1) + BatchNormalization + ReLU + MaxPool + Dropout(0.25)
├── DepthwiseConv2D(3x3) + Conv2D(128, 1x1) + BatchNormalization + ReLU + MaxPool + Dropout(0.25)
├── GlobalAveragePooling2D + Dropout(0.5)
└── Dense(12, softmax) - Output classes
```

## Audio Preprocessing

**Pipeline:**
1. Load 1-second WAV file at 16 kHz (16,000 samples)
2. Compute Short-Time Fourier Transform (STFT)
   - Frame length: 512 samples
   - Hop length: 160 samples
3. Convert to mel spectrogram (40 mel bins, 80-7600 Hz)
4. Apply log scaling
5. Result: (97, 40) MFCC feature matrix

## Results

| Metric | Value |
|--------|-------|
| Test Accuracy | 92.49% |
| Training Accuracy | 89.68% |
| Validation Accuracy | 92.45% |
| Model Parameters | 13,164 |
| Uncompressed Size | 51.42 KB |
| Quantized Size | 26.7 KB |
| Training Time | ~42 minutes (M1 Max GPU) |

## Next Steps

1. **Potential Improvements:**
   - Add data augmentation (time stretching, pitch shifting, background noise)
   - Implement "silence" class with synthetic data
   - Tune hyperparameters for better per-class accuracy
   - Add more target words

## Troubleshooting

### GPU not detected

- Ensure you have Apple Silicon Mac
- Verify `tensorflow-macos==2.15.0` and `tensorflow-metal==1.1.0` are installed
- Check Python version is 3.11 (later versions not supported)

### Live testing not working

- Verify `sounddevice` is installed
- Check microphone permissions in System Settings
- Test with clear audio in quiet environment
- Speak clearly and at normal volume

## References

- [Google Speech Commands Dataset](http://download.tensorflow.org/data/speech_commands_v0.02.tar.gz)
- [Convolutional Neural Networks for Small-footprint Keyword Spotting](https://arxiv.org/abs/1711.07128)
- [TensorFlow Lite for Microcontrollers](https://www.tensorflow.org/lite/microcontrollers)

## License

This project is for educational and demonstration purposes.