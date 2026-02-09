#
#  prepare_dataset.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import tensorflow as tf
import os
import pathlib
import numpy as np

dataset_base_directory = "data/speech_commands"
dataset_directory_path = pathlib.Path(dataset_base_directory)

# Download the dataset (this will take a few minutes, ~2GB)
if not dataset_directory_path.exists():
    tf.keras.utils.get_file(
        "speech_commands_v0.02.tar.gz",
        origin="http://download.tensorflow.org/data/speech_commands_v0.02.tar.gz",
        extract=True,
        cache_dir=".", 
        cache_subdir=dataset_base_directory
    )

# The dataset has 35 word classes, each in its own directory.
# Let’s pick a subset for wake word detection.
# We’ll train on 10 words.
# Plus we need "unknown" (other words) and "silence" classes.

TARGET_WORDS = [
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

# Explore the data
print("Dataset structure:")
for item in dataset_directory_path.iterdir():
    if item.is_dir() and not item.name.startswith("_"):
        count = len(list(item.glob("*.wav")))
        print(f"{item.name}: {count} samples")

