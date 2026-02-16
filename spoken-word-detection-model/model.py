#
#  model.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import tensorflow as tf
from tensorflow import keras

def create_keyword_spotting_model(
        input_shape: tuple[int, int, int],
        number_of_classes: int,
        include_batch_normalization: bool = True,
    ) -> keras.Sequential:
    """
    Create a CNN model for keyword spotting.

    This is a proven architecture for speech commands.
    Based on: https://arxiv.org/abs/1711.07128

    Args:
        input_shape: (time_steps, n_mfcc, channels) e.g., (97, 40, 1)
        number_of_classes: number of output classes
    """
    model = keras.Sequential([
        # Input layer.
        keras.layers.Input(shape=input_shape),

        # First convolutional block.
        keras.layers.Conv2D(32, (3, 3), padding="same"),
        *([ keras.layers.BatchNormalization() ] if include_batch_normalization else []),
        keras.layers.ReLU(),
        keras.layers.MaxPooling2D((2, 2)),
        keras.layers.Dropout(0.25),

        # Second convolutional block.
        keras.layers.Conv2D(64, (3, 3), padding="same"),
        *([ keras.layers.BatchNormalization() ] if include_batch_normalization else []),
        keras.layers.ReLU(),
        keras.layers.MaxPooling2D((2, 2)),
        keras.layers.Dropout(0.25),

        # Third convolutional block.
        keras.layers.Conv2D(128, (3, 3), padding="same"),
        *([ keras.layers.BatchNormalization() ] if include_batch_normalization else []),
        keras.layers.ReLU(),
        keras.layers.MaxPooling2D((2, 2)),
        keras.layers.Dropout(0.25),

        # Global average pooling instead of flatten.
        # This collapses (12, 5, 128) → (128) with zero parameters.
        keras.layers.GlobalAveragePooling2D(),

        # Output layer directly (no intermediate dense layer needed).
        keras.layers.Dropout(0.5),
        keras.layers.Dense(number_of_classes, activation="softmax"),
    ])

    return model


def compile_model(
        model: keras.Sequential,
        learning_rate: float = 0.001,
    ) -> keras.Sequential:
    """Compile the model with optimizer and loss."""
    model.compile(
        optimizer=keras.optimizers.legacy.Adam(learning_rate=learning_rate),
        loss="categorical_crossentropy",
        metrics=["accuracy"],
    )
    return model