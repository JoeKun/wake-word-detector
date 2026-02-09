#
#  data_loader.py
#  wake-word-detector/spoken-word-detection-model
#
#  Created by Joel Lopes Da Silva on 2/8/26.
#  Copyright © 2026 Joel Lopes Da Silva. All rights reserved.
#

import tensorflow as tf
import numpy as np
from pathlib import Path

class SpeechDataset:
    def __init__(
            self, 
            data_directory_path: Path, 
            target_words: list[str], 
            sample_rate: int = 16000
        ):
        """
        Args:
            data_directory_path: path to speech_commands directory as a Path
            target_words: list of words to detect (e.g., ["yes", "no", "stop", "go"])
            sample_rate: audio sample rate
        """
        self.data_directory_path = data_directory_path
        self.target_words = target_words
        self.sample_rate = sample_rate
        
        # Our classes: target words + "unknown" + "silence".
        self.classes = target_words + ["unknown", "silence"]
        self.number_of_classes = len(self.classes)
        
        # Build file lists.
        self.audio_files, self.labels = self._build_file_lists()
    
    def _build_file_lists(self) -> tuple[list[str], list[int]]:
        """Build lists of audio files and their labels."""
        audio_files = []
        labels = []
        
        # Get all word directories.
        all_words = [d.name for d in self.data_directory_path.iterdir() 
                     if d.is_dir() and not d.name.startswith("_")]
        
        for word in all_words:
            word_directory_path = self.data_directory_path / word
            
            # Determine the label.
            if word in self.target_words:
                label = self.target_words.index(word)
            else:
                # Everything else is "unknown".
                label = len(self.target_words)  # unknown class index
            
            # Get all .wav files for this word.
            for wav_file in word_directory_path.glob("*.wav"):
                audio_files.append(str(wav_file))
                labels.append(label)
        
        # Add silence samples (we’ll generate these as needed).
        # For now, just note we’ll need them.
        
        return audio_files, labels
    
    def load_audio(self, file_path: Path) -> tf.Tensor:
        """Load a WAV file and return int16 samples."""
        # Read the WAV file.
        audio_binary = tf.io.read_file(file_path)
        audio, _ = tf.audio.decode_wav(audio_binary, desired_channels=1)
        audio = tf.squeeze(audio, axis=-1)
        
        # Ensure it’s exactly 1 second (16000 samples)
        # Pad or trim as needed.
        audio = self._pad_or_trim(audio, self.sample_rate)
        
        return audio
    
    def _pad_or_trim(self, audio: tf.Tensor, target_length: int) -> tf.Tensor:
        """Pad or trim audio to target length."""
        current_length = tf.shape(audio)[0]
        
        if current_length < target_length:
            # Pad with zeros.
            padding = target_length - current_length
            audio = tf.pad(audio, [[0, padding]])
        elif current_length > target_length:
            # Trim.
            audio = audio[:target_length]
        
        return audio
    
    def extract_mfcc(self, audio: tf.Tensor) -> tf.Tensor:
        """
        Extract MFCC features from audio.
        
        Args:
            audio: float32 tensor of audio samples
        
        Returns:
            mfcc: (time_steps, n_mfcc) tensor
        """
        # Compute STFT.
        stft = tf.signal.stft(
            audio,
            frame_length=512,
            frame_step=160,  # hop length
            fft_length=512
        )
        
        # Magnitude spectrogram.
        magnitude = tf.abs(stft)
        
        # Mel filterbank.
        num_spectrogram_bins = magnitude.shape[-1]
        lower_edge_hertz, upper_edge_hertz = 80.0, 7600.0
        num_mel_bins = 40
        
        linear_to_mel_weight_matrix = tf.signal.linear_to_mel_weight_matrix(
            num_mel_bins, num_spectrogram_bins, self.sample_rate,
            lower_edge_hertz, upper_edge_hertz
        )
        
        mel_spectrogram = tf.tensordot(magnitude, linear_to_mel_weight_matrix, 1)
        
        # Log mel spectrogram.
        log_mel_spectrogram = tf.math.log(mel_spectrogram + 1e-6)
        
        # MFCCs (we’ll use log mel spec directly, which is common for CNNs).
        # If you want actual MFCCs, you’d apply DCT here.
        return log_mel_spectrogram
    
    def get_dataset(self, batch_size: int = 32, shuffle: bool = True) -> tf.data.Dataset:
        """Create a tf.data.Dataset for training."""
        # Create dataset from file paths and labels.
        dataset = tf.data.Dataset.from_tensor_slices((self.audio_files, self.labels))
        
        if shuffle:
            dataset = dataset.shuffle(buffer_size=10000)
        
        # Load and preprocess.
        def process_path(file_path, label):
            audio = self.load_audio(file_path)
            mfcc = self.extract_mfcc(audio)
            # Add channel dimension for CNN.
            mfcc = tf.expand_dims(mfcc, -1)
            # One-hot encode label.
            label_onehot = tf.one_hot(label, self.number_of_classes)
            return mfcc, label_onehot
        
        dataset = dataset.map(process_path, num_parallel_calls=tf.data.AUTOTUNE)
        dataset = dataset.batch(batch_size)
        dataset = dataset.prefetch(tf.data.AUTOTUNE)
        
        return dataset

# Test it.
if __name__ == "__main__":
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
    
    print(f"Total samples: {len(dataset.audio_files)}")
    print(f"Classes: {dataset.classes}")
    print(f"Number of classes: {dataset.number_of_classes}")
    
    # Test loading one batch
    ds = dataset.get_dataset(batch_size=4)
    for mfcc_batch, label_batch in ds.take(1):
        print(f"MFCC batch shape: {mfcc_batch.shape}")
        print(f"Label batch shape: {label_batch.shape}")