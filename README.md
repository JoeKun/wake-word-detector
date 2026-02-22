# Wake Word Detector

A real-time, on-device wake word detection system running entirely on a
[Raspberry Pi Pico 2](https://www.raspberrypi.com/products/raspberry-pi-pico-2/) (RP2350).

No cloud, no external processor — a 150 MHz microcontroller captures audio from an I2S MEMS microphone, extracts log-mel spectrogram features, and runs a quantized neural network, all in real time.

When a word is recognized, an animated pattern lights up on an 8×8 LED matrix.

The system detects ten words: **up, down, right, left, on, off, six, seven, wow, happy**.

## Demo

[Wake Word Detector Demo](https://github.com/user-attachments/assets/3cddd3fc-7f00-4438-90b1-122d503f5e1a)

## How It Works

```
[SPH0645 I2S Microphone]
    → [PIO (Programmable I/O) + DMA (direct memory access)]
        → [log-mel spectrogram feature extraction]
            → [TFLite Micro inference]
                → [8×8 LED matrix display]
```

The firmware runs a sliding window inference loop on core 0:

1. **Capture**: A PIO (programmable I/O) state machine reads I2S audio from the microphone into SRAM via DMA (direct memory access) double-buffering. The CPU is uninvolved during capture.
2. **Feature extraction**: A 1-second audio buffer is converted to a 97×40 log-mel
   spectrogram in Rust.
3. **Inference**: The feature matrix is fed to a fully `int8`-quantized TFLite Micro model
   with CMSIS-NN optimized kernels.
4. **Output**: Detected words trigger display patterns and animations on the 8×8 LED
   matrix, managed by an `OutputController` that sequences instructions over time.

The LED matrix is driven entirely by an interrupt handler that fires every 1 ms, and updates one row per interrupt, resulting in an overall 125 Hz refresh rate. By using interrupts for LED matrix output, the audio pipeline is never blocked by display updates.

## Hardware

| Component | Details |
|-----------|---------|
| Microcontroller | Raspberry Pi Pico 2 (RP2350, Cortex-M33 @ 150 MHz, 520 KB SRAM) |
| Microphone | Adafruit SPH0645LM4H I2S MEMS Microphone Breakout |
| Display | 1588BS 8×8 LED Matrix (common anode) |
| Debug Probe | Raspberry Pi Pico H (RP2040), flashed as picoprobe |

## Wiring

### Microphone (SPH0645 → Pico 2)

| SPH0645 | Pico 2 GPIO | Pico 2 Physical Pin | Notes |
|---------|------------|-------------|-------|
| 3V      | 3V3(OUT)   | Pin 36      | |
| GND     | GND        | Pin 23      | |
| BCLK    | GP13       | Pin 17      | Bit clock |
| LRCL    | GP14       | Pin 19      | Word select — also known as left/right clock |
| DOUT    | GP15       | Pin 20      | Audio data from microphone |
| SEL     | —          | —           | Leave unconnected (defaults to left channel / mono) |

- `BCLK` and `LRCL` must be on consecutive GPIO pins (`LRCL` = `BCLK` + 1) — a constraint of the PIO I2S implementation.
- The SPH0645 outputs data on the falling edge of `BCLK`, which is handled by the PIO program.

### LED Matrix (1588BS → Pico 2)

The display uses row-column scanning: one row (anode) is driven high at a time, while the
appropriate column (cathode) GPIO pins are pulled low to select which LEDs should be turned on.

Resistors go on the column side only — placing them on rows would cause brightness to vary with the number of simultaneously lit LEDs in a row.

**Row pins (anodes) — direct connection, no resistor**

| 1588BS Pin | Row | Pico 2 GPIO | Pico 2 Physical Pin |
|-----------|-----|------------|-------------|
| Pin 9     | 1   | GP5        | Pin 7  |
| Pin 14    | 2   | GP6        | Pin 9  |
| Pin 8     | 3   | GP7        | Pin 10 |
| Pin 12    | 4   | GP8        | Pin 11 |
| Pin 1     | 5   | GP9        | Pin 12 |
| Pin 7     | 6   | GP10       | Pin 14 |
| Pin 2     | 7   | GP11       | Pin 15 |
| Pin 5     | 8   | GP12       | Pin 16 |

**Column pins (cathodes) — each through a 220Ω resistor**

| 1588BS Pin | Column | Pico 2 GPIO | Pico 2 Physical Pin |
|-----------|--------|------------|-------------|
| Pin 13    | 1      | GP16       | Pin 21 |
| Pin 3     | 2      | GP17       | Pin 22 |
| Pin 4     | 3      | GP18       | Pin 24 |
| Pin 10    | 4      | GP19       | Pin 25 |
| Pin 6     | 5      | GP20       | Pin 26 |
| Pin 11    | 6      | GP21       | Pin 27 |
| Pin 15    | 7      | GP22       | Pin 29 |
| Pin 16    | 8      | GP26       | Pin 31 |

### Status LED

GP25 (onboard Pico 2 LED) — lit when the detector is active.

## Repository Structure

```
wake-word-detector/
├── src/
│   ├── main.rs                     Entry point, hardware initialization, task launch
│   ├── detectable_words.rs         Word enum, display pattern mapping
│   ├── hardware_resources.rs       Peripheral initialization (clocks, PIO, DMA, GPIO)
│   ├── audio/
│   │   ├── task.rs                 Audio capture and inference loop
│   │   ├── features.rs             Log-mel spectrogram feature extraction (int8 quantization)
│   │   └── mel_constants.rs        Pre-computed Hann window and mel filterbank (generated)
│   ├── inference/
│   │   ├── tflite.rs               Rust FFI declarations for the TFLite Micro C++ wrapper
│   │   ├── tflite_wrapper.cc       Thin C++ wrapper exposing TFLite Micro via a C interface
│   │   └── tflite_wrapper.h        C-compatible header for the wrapper
│   └── output/
│       ├── controller.rs           OutputController — interrupt-driven output orchestration
│       ├── instruction.rs          OutputInstruction enum (display, LED, wait, clear)
│       ├── sequence.rs             OutputSequence — ordered, timed instruction queue
│       ├── led_matrix.rs           8×8 LED matrix driver (interrupt-driven row scanning)
│       └── display_patterns.rs     8×8 boolean pixel patterns for each detected word
├── build-configuration/
│   └── build.rs                    Compiles the C++ wrapper, links libtensorflow-microlite
├── spoken-word-detection-model/    Python ML training pipeline (see sub-README)
└── README.md
```


## Building the Firmware

### Prerequisites

#### Rust and the embedded target

Install Rust via [Rustup](https://rust-lang.org/learn/get-started/), then add the Cortex-M33 target:

```bash
rustup target add thumbv8m.main-none-eabihf
```

#### ARM cross-compiler

The C++ TFLite wrapper requires `arm-none-eabi-g++`. On macOS with Homebrew:

```bash
brew install --cask gcc-arm-embedded
```

#### GNU Make

Building TFLite Micro requires GNU Make ≥ 3.82. On macOS with Homebrew:

```bash
brew install make
```

Homebrew installs this as `gmake` to avoid shadowing the system `make`.

#### probe-rs

[probe-rs](https://probe.rs/) is used to flash the firmware and stream `defmt` logs.

If you have the [Raspberry Pi Pico Visual Studio Code extension](https://marketplace.visualstudio.com/items?itemName=raspberry-pi.raspberry-pi-pico)
installed, `probe-rs` is likely already on your system.

Otherwise, install it via Cargo:

```bash
cargo install probe-rs-tools --locked
```

### Build the TFLite Micro static library

The firmware links against a pre-built `libtensorflow-microlite.a`.

Clone `tflite-micro` and build the library once with CMSIS-NN enabled:

```bash
cd /path/to/wake-word-detector
cd ..
git clone https://github.com/tensorflow/tflite-micro
cd tflite-micro
```

The build system uses Python scripts that require `numpy` and `pillow`. Set up a virtual environment:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install numpy pillow
```

Then build the library (with the `venv` active):

```bash
gmake -f tensorflow/lite/micro/tools/make/Makefile \
    TARGET=cortex_m_generic                        \
    TARGET_ARCH=cortex-m33                         \
    OPTIMIZED_KERNEL_DIR=cmsis_nn                  \
    BUILD_TYPE=release_with_logs                   \
    microlite
```

The expected library path is set in `build-configuration/build.rs`.

Once this is successfully built, you can deactivate the virtual environment:

```bash
deactivate
```

### Build and flash

```bash
cd /path/to/wake-word-detector
cargo run --release
```

The `.cargo/config.toml` configures `probe-rs` as the runner, so `cargo run` builds,
flashes, and attaches a `defmt` log stream in one step.

## Model Training

The `spoken-word-detection-model/` directory contains the full Python pipeline for
training, evaluating, quantizing, and deploying the model.

See [`spoken-word-detection-model/README.md`](spoken-word-detection-model/README.md) for setup and usage instructions.

**Summary:**
- Dataset: [Google Speech Commands](http://download.tensorflow.org/data/speech_commands_v0.02.tar.gz) (~105,000 samples, 35 word classes).
- Target vocabulary: up, down, right, left, on, off, six, seven, wow, happy (+ unknown, silence).
- Architecture:
	- `Conv2D` block
	- two depthwise separable `Conv2D` blocks
	- `GlobalAveragePooling2D`
	- `Dense(12)`
- `BatchNormalization` fused into `Conv2D` weights before quantization.
- Full `int8` quantization (weights and activations) using representative dataset calibration.

## Performance

| Metric | Value |
|--------|-------|
| Test accuracy | 92.49% |
| Model parameters | 13,164 |
| Model size (int8 quantized) | 26.7 KB |
| Tensor arena usage | 159 KB / 192 KB |
| Feature extraction | ~50 ms / buffer |
| Inference | ~182 ms / buffer |
| Effective detection rate | ~4 detections / second |

### Optimizations

Inference performance was substantially improved by a factor of 27.7x over the initial implementation (5,058 ms → 182 ms)

Here are some of the key optimizations that led to this impressive speedup.
 - Enable usage of CMSIS-NN kernels with TFLite Micro.
 - Fuse `BatchNormalization` into preceding `Conv2D` weights to eliminate unnecessary multiplay-add operations at inference time.
 - Optimize model architecture for inference speed by switching second and third `Conv2D` blocks to using `DepthwiseConv2D` followed by a simpler `Conv2D` block.

## Implementation Notes

### TFLite Micro via direct C FFI

Rather than using bindgen, a thin C++ wrapper (`tflite_wrapper.cc`) exposes the TFLite Micro C++ API through a C-compatible interface callable from Rust with no generated bindings.

### Interrupt-driven LED scanning

The 8×8 matrix is scanned entirely from an interrupt handler (1ms period, one row per interrupt). This keeps the audio pipeline fully unblocked and eliminates the XIP flash cache contention that arises when a second core executes code concurrently.

### Log-mel spectrogram feature extraction in Rust

The pipeline — FFT, mel filterbank application, log scaling, and int8 quantization — is implemented from scratch in Rust.

The Hann window and mel filterbank weights are pre-computed constants generated by `spoken-word-detection-model/generate_rust_constants.py` and compiled in at build time.

### BatchNormalization fusion

BatchNormalization parameters are algebraically folded into the preceding `Conv2D` weights (`fuse_batch_normalization.py`) before quantization, eliminating separate multiply-add operations at inference time with no loss in accuracy.

### Output sequence instruction queue

Detected words trigger an `OutputSequence` — a fixed-capacity array of `OutputInstruction` values (display patterns, LED state changes, timed waits) that the `OutputController` drains from its interrupt handler.

This makes multi-step animations (e.g. the "six seven" celebration sequence) expressible as a simple declarative list rather than ad-hoc state machines.

## References

- [Raspberry Pi Pico 2 Datasheet](https://datasheets.raspberrypi.com/pico/pico-2-datasheet.pdf)
- [RP2350 Datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- [Adafruit SPH0645 I2S MEMS Microphone](https://learn.adafruit.com/adafruit-i2s-mems-microphone-breakout)
- [SPH0645LM4H Datasheet](https://cdn-shop.adafruit.com/product-files/3421/i2S+Datasheet.PDF)
- [1588BS LED Matrix Datasheet](https://www.hestore.eu/en/prod_getfile.php?id=15694)
- [TensorFlow Lite for Microcontrollers](https://www.tensorflow.org/lite/microcontrollers)
- [Google Speech Commands Dataset](http://download.tensorflow.org/data/speech_commands_v0.02.tar.gz)
- [Keyword Spotting for Microcontrollers (Warden, 2018)](https://arxiv.org/abs/1711.07128)
- [Raspberry Pi Forum: PIO for SPH0645](https://forums.raspberrypi.com/viewtopic.php?t=306882)
