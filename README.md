# Wake Word Detector

A wake word detection system using a CNN model trained on the Google Speech Commands dataset,
targeting deployment on a Raspberry Pi Pico 2 (RP2350) with an I2S MEMS microphone.

## Hardware

- **Microcontroller:** Raspberry Pi Pico 2 (RP2350)
- **Microphone:** Adafruit SPH0645LM4H I2S MEMS Microphone Breakout
- **Debug Probe:** Raspberry Pi Pico H (RP2040), repurposed as picoprobe

## Wiring: SPH0645 to Raspberry Pi Pico 2

| SPH0645 Pin | Pico 2 Pin | Physical Pin | Notes |
|-------------|------------|-------------|-------|
| **3V** | 3V3(OUT) | Pin 36 | 3.3V power supply |
| **GND** | GND | Pin 23 | Common ground |
| **BCLK** | GP13 | Pin 17 | Bit clock |
| **LRCL** | GP14 | Pin 19 | Word select (BCLK + 1) |
| **DOUT** | GP15 | Pin 20 | Audio data from mic |
| **SEL** | -- | -- | Leave unconnected (defaults to left channel / mono) |

```
Raspberry Pi Pico 2                    SPH0645 Breakout
==================                    ================

3V3(OUT) (Pin 36) ────────────────── 3V
GND      (Pin 23) ────────────────── GND
GP13     (Pin 17) ────────────────── BCLK
GP14     (Pin 19) ────────────────── LRCL
GP15     (Pin 20) ────────────────── DOUT
                          (nothing) ── SEL
```

### Wiring Notes

- **BCLK and LRCL must be consecutive GPIOs** (LRCL = BCLK + 1) -- PIO constraint on RP2350.
- **DOUT can be any GPIO.**
- **Power at 3.3V only** -- never connect to VBUS (5V).
- **No pull-up/pull-down resistors needed.**
- **Minimum ~1 MHz BCLK** -- 16 kHz sample rate with 32-bit stereo frames gives 1.024 MHz.
- **SPH0645 outputs data on the falling edge** of BCLK (non-standard) -- PIO program must account for this.
- **Bottom-ported mic** -- sound hole is on the underside of the breakout; don't block it.

### References

- [Adafruit I2S MEMS Mic - CircuitPython Wiring & Test](https://adafruit-playground.com/u/relic_se/pages/adafruit-i2s-mems-microphone-breakout-circuitpython-wiring-test)
- [Adafruit I2S MEMS Mic Pinouts Guide](https://learn.adafruit.com/adafruit-i2s-mems-microphone-breakout/pinouts)
- [SPH0645LM4H Datasheet](https://cdn-shop.adafruit.com/product-files/3421/i2S+Datasheet.PDF)
- [Raspberry Pi Forum: PIO for SPH0645](https://forums.raspberrypi.com/viewtopic.php?t=306882)
- [vijaymarupudi/sph0645-pico-troubleshooting (GitHub)](https://github.com/vijaymarupudi/sph0645-pico-troubleshooting)
- [Raspberry Pi Pico 2 Datasheet](https://datasheets.raspberrypi.com/pico/pico-2-datasheet.pdf)
