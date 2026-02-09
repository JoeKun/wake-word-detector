#
#  train.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import tensorflow as tf
from tensorflow import keras
from pathlib import Path
import os
from data_loader import SpeechDataset
from train_test_split import split_dataset
from model import create_keyword_spotting_model, compile_model

def train_model() -> tuple[
        keras.Sequential,                # model
        keras.callbacks.History,         # training history
        tuple[list[str], list[int]]      # test data (files, labels)
    ]:
    # 1. Load dataset.
    print("Loading dataset...")
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
        ]
    )
    
    # 2. Split data.
    (train_files, train_labels), (validation_files, validation_labels), (test_files, test_labels) = \
        split_dataset(dataset.audio_files, dataset.labels)
    
    # 3. Create TensorFlow datasets.
    dataset.audio_files = train_files
    dataset.labels = train_labels
    train_dataset = dataset.get_dataset(batch_size=32, shuffle=True)
    
    dataset.audio_files = validation_files
    dataset.labels = validation_labels
    validation_dataset = dataset.get_dataset(batch_size=32, shuffle=False)
    
    # 4. Get input shape from one batch.
    for mfcc_batch, _ in train_dataset.take(1):
        input_shape = mfcc_batch.shape[1:]  # (time_steps, n_mfcc, channels)
        print(f"Input shape: {input_shape}")
    
    # 5. Create model.
    print("Creating model...")
    model = create_keyword_spotting_model(input_shape, dataset.number_of_classes)
    model = compile_model(model, learning_rate=0.001)
    model.summary()
    
    # 6. Callbacks.
    callbacks = [
        # Early stopping: stop if validation loss doesn’t improve.
        keras.callbacks.EarlyStopping(
            monitor="val_loss",
            patience=10,
            restore_best_weights=True
        ),
        # Reduce learning rate when validation loss plateaus.
        keras.callbacks.ReduceLROnPlateau(
            monitor="val_loss",
            factor=0.5,
            patience=5,
            min_lr=1e-6
        ),
        # Save best model.
        keras.callbacks.ModelCheckpoint(
            "models/best_model.keras",
            monitor="val_accuracy",
            save_best_only=True,
            verbose=1
        ),
        # TensorBoard logging.
        keras.callbacks.TensorBoard(
            log_dir="logs",
            histogram_freq=1
        ),
    ]
    
    # 7. Train.
    print("Starting training...")
    os.makedirs("models", exist_ok=True)
    
    history = model.fit(
        train_dataset,
        validation_data=validation_dataset,
        epochs=50,  # Early stopping will likely stop before this
        callbacks=callbacks,
        verbose=1
    )
    
    # 8. Save final model.
    model.save("models/final_model.keras")
    
    return model, history, (test_files, test_labels)

if __name__ == "__main__":
    model, history, test_data = train_model()