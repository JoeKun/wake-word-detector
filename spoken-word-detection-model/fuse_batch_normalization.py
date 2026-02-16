#
#  fuse_batch_normalization.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/15/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import numpy as np
from tensorflow import keras

from model import create_keyword_spotting_model

# For each (Conv2D, BatchNormalization) pair, BatchNormalization computes:
#
#   y = gamma * (x - mean) / sqrt(variance + epsilon) + beta
#
# Since x = W * input + b (the Conv2D output), we can substitute and rearrange:
#
#   y = (gamma / sqrt(variance + epsilon)) * W * input
#       + (b - mean) * (gamma / sqrt(variance + epsilon)) + beta
#
# So the fused weights are:
#
#   std = np.sqrt(variance + epsilon)
#   scale = gamma / std                          # shape: [C_out]
#   new_W = W * scale                            # broadcast over output channel dimension
#   new_b = (b - mean) * scale + beta            # per output channel

def fuse_conv2d_and_batch_normalization(
        conv2d_layer: keras.layers.Conv2D,
        batch_normalization_layer: keras.layers.BatchNormalization,
    ) -> tuple[np.ndarray, np.ndarray]:
    """
    Fuse Conv2D and BatchNormalization into a single Conv2D.
    """
    weights, bias = conv2d_layer.get_weights()
    gamma, beta, moving_mean, moving_variance = batch_normalization_layer.get_weights()

    epsilon = batch_normalization_layer.epsilon
    standard_deviation = np.sqrt(moving_variance + epsilon)
    scale = gamma / standard_deviation

    fused_weights = weights * scale         # broadcasts: [H, W, C_in, C_out] * [C_out]
    fused_bias = (bias - moving_mean) * scale + beta

    return fused_weights, fused_bias

def create_fused_model(model: keras.Sequential) -> keras.Sequential:
    """
    Create a new model with BatchNormalization fused into Conv2D.
    """
    input_shape = model.input_shape[1:]     # drop batch dimension

    # Collect fused weights for each conv block.
    conv_bn_pairs = []
    for i, layer in enumerate(model.layers):
        if i > 0 and isinstance(layer, keras.layers.BatchNormalization):
            previous_layer = model.layers[i - 1]
            if isinstance(previous_layer, keras.layers.Conv2D):
                conv_bn_pairs.append((previous_layer, layer))

    fused_weights = [
        fuse_conv2d_and_batch_normalization(conv, bn)
        for conv, bn in conv_bn_pairs
    ]

    # Build fused model - same architecture but no BatchNormalization layers.
    fused_model = create_keyword_spotting_model(
        input_shape=input_shape, 
        number_of_classes=12, 
        include_batch_normalization=False,
    )

    # Set fused weights on each Conv2D.
    fused_conv_layers = [
        layer
        for layer in fused_model.layers if isinstance(layer, keras.layers.Conv2D)
    ]
    for conv_layer, (w, b) in zip(fused_conv_layers, fused_weights):
        conv_layer.set_weights([w, b])

    # Copy the Dense layer weights unchanged.
    original_dense = model.layers[-1]
    fused_dense = fused_model.layers[-1]
    fused_dense.set_weights(original_dense.get_weights())

    return fused_model

def verify_fusion(
        original_model: keras.Sequential,
        fused_model: keras.Sequential,
    ):
    """
    Verify that the fused model produces the same outputs.
    """
    test_input = np.random.randn(1, 97, 40, 1).astype(np.float32)

    original_output = original_model.predict(test_input, verbose=0)
    fused_output = fused_model.predict(test_input, verbose=0)

    max_difference = np.max(np.abs(original_output - fused_output))
    print(f"Max output difference: {max_difference:.8f}")

    if max_difference < 1e-5:
        print("Fusion verified: outputs match.")
    else:
        print("WARNING: outputs differ significantly!")

if __name__ == "__main__":
    print("Loading model…")
    model = keras.models.load_model("models/best_model.keras")
    model.summary()

    print("\nFusing BatchNormalization into Conv2D…")
    fused_model = create_fused_model(model)
    fused_model.summary()

    print("\nVerifying fusion…")
    verify_fusion(model, fused_model)

    print("\nSaving fused model…")
    fused_model.save("models/best_model_fused.keras")
    print("Saved to models/best_model_fused.keras")