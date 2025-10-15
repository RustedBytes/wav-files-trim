use anyhow::{Context, Result};
use clap::Parser;
use hound::{SampleFormat, WavReader, WavWriter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// CLI arguments for the wav-files-trim tool.
#[derive(Parser, Debug)]
#[command(
    name = "wav-files-trim",
    about = "Recursively trims silence from the start and end of WAV files in a directory."
)]
struct Args {
    /// Input directory containing WAV files (processed recursively).
    input_dir: String,

    /// Output directory for trimmed WAV files (mirrors input structure).
    output_dir: String,

    /// Silence detection threshold in dBFS (default: -50.0; higher values trim more aggressively).
    #[arg(short, long, default_value_t = -50.0)]
    threshold: f64,
}

/// Trims leading and trailing silence from a WAV file based on RMS over a sliding window.
///
/// # Arguments
///
/// * `input_path` - Path to the input WAV file.
/// * `output_path` - Path to write the trimmed WAV file.
/// * `threshold_db` - dBFS threshold for silence detection (negative value).
///
/// # Errors
///
/// Returns an error if the file format is unsupported or I/O fails.
pub fn trim_wav(input_path: &Path, output_path: &Path, threshold_db: f64) -> Result<()> {
    let mut reader = WavReader::open(input_path).context("Failed to open input WAV file")?;
    let spec = reader.spec();

    // Validate format as per project context (mono, 16-bit PCM, 16kHz).
    if spec.channels != 1
        || spec.sample_rate != 16_000
        || spec.bits_per_sample != 16
        || spec.sample_format != SampleFormat::Int
    {
        anyhow::bail!("Unsupported WAV format: expected mono 16-bit PCM at 16kHz");
    }

    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, hound::Error>>()
        .context("Failed to read samples")?;

    let trimmed_samples = trim_samples(&samples, threshold_db, 800)?; // 50ms window at 16kHz

    let mut writer =
        WavWriter::create(output_path, spec).context("Failed to create output WAV file")?;
    for &sample in &trimmed_samples {
        writer
            .write_sample(sample)
            .context("Failed to write sample")?;
    }
    writer.finalize().context("Failed to finalize WAV writer")?;

    Ok(())
}

/// Computes the RMS value of a slice of i16 samples.
fn rms(chunk: &[i16]) -> f64 {
    if chunk.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = chunk.iter().map(|&s| (s as f64).powi(2)).sum();
    (sum_sq / chunk.len() as f64).sqrt()
}

/// Trims leading/trailing silence from samples using RMS-based detection over a fixed window.
fn trim_samples(samples: &[i16], threshold_db: f64, window_size: usize) -> Result<Vec<i16>> {
    let len = samples.len();
    if len == 0 {
        return Ok(Vec::new());
    }

    let full_scale = 32768.0f64;
    let threshold_linear = 10f64.powf(threshold_db / 20.0);
    let threshold_rms = threshold_linear * full_scale;

    // Find start trim point: first window with RMS above threshold.
    let mut start_trim = len;
    for i in (0..len).step_by(window_size) {
        let chunk_end = (i + window_size).min(len);
        let chunk_rms = rms(&samples[i..chunk_end]);
        if chunk_rms > threshold_rms {
            start_trim = i;
            break;
        }
    }

    // Find end trim point: last window with RMS above threshold.
    let mut end_trim = 0;
    for i in (0..=len).rev().step_by(window_size) {
        let chunk_start = (i.saturating_sub(window_size)).max(0);
        let chunk_rms = rms(&samples[chunk_start..i]);
        if chunk_rms > threshold_rms {
            end_trim = i;
            break;
        }
    }

    let trimmed = if start_trim < end_trim {
        samples[start_trim..end_trim].to_vec()
    } else {
        Vec::new()
    };

    Ok(trimmed)
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_dir = Path::new(&args.input_dir);
    let output_dir = Path::new(&args.output_dir);

    if !input_dir.exists() {
        anyhow::bail!("Input directory does not exist: {}", args.input_dir);
    }

    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let mut processed = 0;
    for entry in WalkDir::new(input_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|ext| ext.to_str()) == Some("wav")
        {
            let rel_path = entry
                .path()
                .strip_prefix(input_dir)
                .context("Failed to compute relative path")?;
            let output_path: PathBuf = output_dir.join(rel_path);

            // Ensure parent directories exist.
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).context("Failed to create output subdirectory")?;
            }

            if let Err(e) = trim_wav(entry.path(), &output_path, args.threshold) {
                eprintln!("Error processing {}: {}", entry.path().display(), e);
            } else {
                processed += 1;
            }
        }
    }

    println!("Processed {} WAV files.", processed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_silence() {
        let chunk = vec![0i16; 10];
        assert_eq!(rms(&chunk), 0.0);
    }

    #[test]
    fn test_rms_full_scale() {
        let chunk = vec![32767i16; 10];
        let rms_val = rms(&chunk);
        assert!((rms_val - 32767.0).abs() < 1e-6);
    }

    #[test]
    fn test_trim_all_silence() {
        let samples = vec![0i16; 1000];
        let threshold_db = -50.0;
        let window_size = 100;
        let trimmed = trim_samples(&samples, threshold_db, window_size).unwrap();
        assert_eq!(trimmed.len(), 0);
    }

    #[test]
    fn test_trim_leading_trailing_silence() {
        let silence_len = 800;
        let signal = vec![1000i16; 400]; // Above threshold RMS.
        let samples = vec![0i16; silence_len]
            .into_iter()
            .chain(signal.clone())
            .chain(vec![0i16; silence_len])
            .collect::<Vec<_>>();
        let threshold_db = -40.0; // Threshold such that RMS(1000 over 400) > threshold.
        let window_size = 200;
        let trimmed = trim_samples(&samples, threshold_db, window_size).unwrap();
        assert_eq!(trimmed.len(), signal.len());
        assert_eq!(trimmed, signal);
    }

    #[test]
    fn test_trim_no_silence() {
        let samples = vec![1000i16; 1000];
        let threshold_db = -60.0;
        let window_size = 100;
        let trimmed = trim_samples(&samples, threshold_db, window_size).unwrap();
        assert_eq!(trimmed.len(), samples.len());
    }
}
