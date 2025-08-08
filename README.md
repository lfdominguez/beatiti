# beatiti (WIP)

A Rust application that detects the BPM (Beats Per Minute) of audio playing on your system using `Pipewire` for audio capture and `aubio` for beat detection.

## Features

- Real-time BPM detection from system audio
- Uses Pipewire to capture audio from any source on your system
- Implements aubio's tempo detection algorithm for accurate beat analysis
- Smooths BPM readings with a running average
- Includes octave error correction for more reliable results
- Displays confidence levels for detected BPM values

## Requirements

- Linux system with Pipewire audio server
- Rust toolchain (2024 edition)
- aubio library installed on your system

## Installation

1. Make sure you have the Rust toolchain installed:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Install the aubio library:
   ```
   # Debian/Ubuntu
   sudo apt install libaubio-dev
   
   # Fedora
   sudo dnf install aubio-devel
   
   # Arch Linux
   sudo pacman -S aubio
   ```

3. Clone this repository:
   ```
   git clone https://github.com/yourusername/pipe-beat-detector.git
   cd pipe-beat-detector
   ```

4. Build the application:
   ```
   cargo build --release
   ```

## Usage

Run the application:

```
cargo run --release
```

The application will:
1. Connect to your system's audio through Pipewire
2. Capture audio in real-time
3. Process the audio to detect beats
4. Display the current BPM with confidence levels

The BPM is displayed in the terminal with the following information:
- Corrected BPM (after smoothing and octave correction)
- Raw BPM (direct from aubio)
- Confidence level (how certain the algorithm is about the detection)
- Number of samples in the BPM history

## How It Works

1. **Audio Capture**: Uses Pipewire to capture system audio
2. **Sample Processing**: 
   - Converts stereo audio to mono
   - Buffers audio samples in chunks
   - Processes audio in windows of 1024 samples with 50% overlap
3. **BPM Detection**:
   - Uses aubio's Tempo detector with HFC (High Frequency Content) onset detection
   - Filters out unreasonable BPM values (outside 30-300 BPM range)
4. **BPM Smoothing**:
   - Maintains a history of recent BPM readings
   - Calculates a running average for stability
5. **Octave Error Correction**:
   - Detects and corrects common octave errors (halving/doubling of actual tempo)
   - Uses confidence levels to determine when correction is needed

## Configuration

The application uses the following default settings:

- Buffer size: 1024 samples
- Hop size: 512 samples (50% overlap)
- BPM history size: 5 readings
- Silence threshold: -40.0 dB
- Onset detection threshold: 0.1
- Valid BPM range: 30-300 BPM

## License

MIT

## Acknowledgments

- [Pipewire](https://pipewire.org/) - For the audio capture API
- [aubio](https://aubio.org/) - For the audio beat tracking algorithms
