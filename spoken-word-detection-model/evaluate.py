#
#  evaluate.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import tensorflow as tf
from tensorflow import keras
import numpy as np
from sklearn.metrics import classification_report, confusion_matrix
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path
from data_loader import SpeechDataset

def evaluate_model(
        model_path: str, 
        test_files: list[str], 
        test_labels: list[int], 
        dataset: SpeechDataset
    ) -> tuple[float, float]:
    """
    Evaluate model on test set and determine confidence thresholds.
    
    Target metrics:
    - Overall accuracy: >85% (good), >90% (excellent)
    - Per-class accuracy: >80% for each target word
    - False positive rate for "unknown": <5%
    """
    # Load model.
    model = keras.models.load_model(model_path)
    
    # Create test dataset.
    dataset.audio_files = test_files
    dataset.labels = test_labels
    test_ds = dataset.get_dataset(batch_size=32, shuffle=False)
    
    # Get predictions.
    print("Running predictions on test set...")
    y_true = []
    y_pred = []
    confidences = []
    
    for mfcc_batch, label_batch in test_ds:
        predictions = model.predict(mfcc_batch, verbose=0)
        
        y_true.extend(np.argmax(label_batch.numpy(), axis=1))
        y_pred.extend(np.argmax(predictions, axis=1))
        confidences.extend(np.max(predictions, axis=1))
    
    y_true = np.array(y_true)
    y_pred = np.array(y_pred)
    confidences = np.array(confidences)
    
    # Overall accuracy.
    accuracy = np.mean(y_true == y_pred)
    print(f"\n{'='*50}")
    print(f"OVERALL TEST ACCURACY: {accuracy:.4f} ({accuracy*100:.2f}%)")
    print(f"{'='*50}\n")
    
    # Classification report.
    print("Per-class metrics:")
    print(classification_report(
        y_true, y_pred,
        target_names=dataset.classes,
        labels=list(range(dataset.number_of_classes)), 
        digits=4
    ))
    
    # Confusion matrix.
    cm = confusion_matrix(y_true, y_pred)
    plt.figure(figsize=(10, 8))
    sns.heatmap(cm, annot=True, fmt="d", cmap="Blues",
                xticklabels=dataset.classes,
                yticklabels=dataset.classes)
    plt.title("Confusion Matrix")
    plt.ylabel("True Label")
    plt.xlabel("Predicted Label")
    plt.tight_layout()
    plt.savefig("confusion_matrix.png")
    print("Confusion matrix saved to confusion_matrix.png")
    
    # Confidence threshold analysis.
    print("\n" + "="*50)
    print("CONFIDENCE THRESHOLD ANALYSIS")
    print("="*50)
    
    for threshold in [0.5, 0.6, 0.7, 0.8, 0.9]:
        # Apply threshold.
        pred_with_threshold = y_pred.copy()
        # If confidence below threshold, predict "unknown".
        unknown_idx = len(dataset.target_words)
        pred_with_threshold[confidences < threshold] = unknown_idx
        
        acc = np.mean(y_true == pred_with_threshold)
        
        # Calculate false positive rate (predicting a word when it’s unknown).
        unknown_mask = y_true == unknown_idx
        false_positives = np.sum((pred_with_threshold != unknown_idx) & unknown_mask)
        fpr = false_positives / np.sum(unknown_mask) if np.sum(unknown_mask) > 0 else 0
        
        print(f"\nThreshold: {threshold:.2f}")
        print(f"  Accuracy: {acc:.4f}")
        print(f"  False Positive Rate: {fpr:.4f}")
    
    # Determine recommended threshold.
    # We want high accuracy but low false positive rate.
    recommended_threshold = 0.7
    print(f"\n{'='*50}")
    print(f"RECOMMENDED CONFIDENCE THRESHOLD: {recommended_threshold}")
    print(f"{'='*50}\n")
    
    # Check if we meet our targets.
    print("TARGET METRICS:")
    print(f"✓ Overall accuracy >85%: {'PASS' if accuracy > 0.85 else 'FAIL'}")
    print(f"✓ Overall accuracy >90%: {'PASS' if accuracy > 0.90 else 'EXCELLENT!' if accuracy > 0.90 else 'Not yet'}")
    
    return accuracy, recommended_threshold

if __name__ == "__main__":
    from data_loader import SpeechDataset
    from train_test_split import split_dataset
    
    # Load dataset to get test split
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
    
    _, _, (test_files, test_labels) = split_dataset(dataset.audio_files, dataset.labels)
    
    # Evaluate
    evaluate_model("models/best_model.keras", test_files, test_labels, dataset)