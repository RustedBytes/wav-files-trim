# wav-files-trim

A command-line tool for recursively trimming silence from the beginning and end of WAV audio files. Designed for preprocessing speech audio to enhance separation and chunking by removing dead air. Supports mono 16-bit PCM WAV files at 16kHz sample rate.

Part of the [RustedBytes](https://github.com/RustedBytes) organization.

## Features

- **Recursive Processing**: Scans input directory and subdirectories for `.wav` files.
- **Structure Preservation**: Mirrors the input directory structure in the output folder.
- **Silence Detection**: Uses RMS-based thresholding over sliding windows (50ms default) for robust trim detection.
- **Configurable Threshold**: Adjustable dBFS threshold (default: -50dBFS) for aggressive or conservative trimming.
- **Format Validation**: Ensures files match expected specs (mono, 16-bit, 16kHz).
- **Error Resilience**: Continues processing on per-file errors, logging issues to stderr.

## Installation

### From Source

1. Clone the repository:
   ```bash
   git clone https://github.com/RustedBytes/wav-files-trim.git
   cd wav-files-trim
   ```

2. Build and install:
   ```bash
   cargo install --path .
   ```

Requires Rust 1.75+ (stable channel). See [Rust installation guide](https://www.rust-lang.org/tools/install) if needed.

## Usage

```bash
wav-files-trim [OPTIONS] <INPUT_DIR> <OUTPUT_DIR>
```

### Arguments

- `<INPUT_DIR>`: Path to the input directory containing WAV files.
- `<OUTPUT_DIR>`: Path to the output directory for trimmed files.

### Options

- `-t, --threshold <THRESHOLD>`: Silence detection threshold in dBFS (default: `-50.0`). Higher (less negative) values trim more aggressively.

Run `wav-files-trim --help` for full details.

## Examples

### Basic Trimming

Trim silence from all WAV files in `input/` and save to `output/`:

```bash
wav-files-trim input/ output/
```

This processes recursively, creating `output/subdir/file_trimmed.wav` for each input.

### Custom Threshold

For more aggressive trimming (e.g., -40dBFS):

```bash
wav-files-trim input/ output/ --threshold -40.0
```

### Handling Errors

If a file has an unsupported format, it skips with a warning:

```
Error processing input/bad.wav: Unsupported WAV format: expected mono 16-bit PCM at 16kHz
Processed 42 WAV files.
```

## Configuration

No additional config files; all options are CLI-based for simplicity. Threshold tuning:
- `-60dBFS`: Conservative (trims only very quiet silence).
- `-50dBFS`: Balanced for speech (default).
- `-40dBFS`: Aggressive (removes more background noise).

Test on sample files to dial in.

## Testing

Run the test suite:

```bash
cargo test
```

Includes unit tests for RMS calculation, trim logic (all silence, leading/trailing, no trim), and edge cases.

## Contributing

Contributions welcome!

1. Fork the repo and create a feature branch (`git checkout -b feat/amazing-feature`).
2. Commit changes (`git commit -m 'Add amazing feature'`).
3. Push (`git push origin feat/amazing-feature`).
4. Open a Pull Request.

## License

MIT License. See [LICENSE](LICENSE) for details.
