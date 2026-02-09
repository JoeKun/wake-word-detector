#
#  train_test_split.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

from sklearn.model_selection import train_test_split
import numpy as np

def split_dataset(
    audio_files: list[str], 
    labels: list[int], 
    validation_size: float = 0.15, 
    test_size: float = 0.15, 
    random_state: int = 42
) -> tuple[
        tuple[list[str], list[int]], # training set (files and labels)
        tuple[list[str], list[int]], # validation set (files and labels)
        tuple[list[str], list[int]], # testing set (files and labels)
    ]:
    """
    Split dataset into train/val/test sets.
    
    Standard split: 70% train, 15% validation, 15% test
    """
    # First split: separate out test set
    train_validation_files, test_files, train_validation_labels, test_labels = train_test_split(
        audio_files, labels,
        test_size=test_size,
        random_state=random_state,
        stratify=labels  # Ensure balanced classes
    )
    
    # Second split: separate train and validation
    validation_size_adjusted = validation_size / (1 - test_size)  # Adjust for already removed test
    train_files, validation_files, train_labels, validation_labels = train_test_split(
        train_validation_files, train_validation_labels,
        test_size=validation_size_adjusted,
        random_state=random_state,
        stratify=train_validation_labels
    )
    
    print(f"Train set: {len(train_files)} samples")
    print(f"Validation set: {len(validation_files)} samples")
    print(f"Test set: {len(test_files)} samples")
    
    return (train_files, train_labels), (validation_files, validation_labels), (test_files, test_labels)