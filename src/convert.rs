use base64::prelude::*;
use clap::Parser;
use hound::{SampleFormat, WavSpec};
use scanf::scanf;
use std::fs::File;
use std::io::{Write, read_to_string};
use std::mem::forget;
use std::str::FromStr;

#[derive(clap::Subcommand)]
enum Commands {
    CSV {
        file: std::path::PathBuf,
        #[arg(short)]
        output: std::path::PathBuf,
    },
    WAV {
        file: std::path::PathBuf,
        #[arg(short)]
        output: std::path::PathBuf,
    },

    ToChunks {
        file: std::path::PathBuf,
        #[arg(short)]
        output: std::path::PathBuf,
    },

    FromChunks {
        file: std::path::PathBuf,
        #[arg(short)]
        output_prefix: std::path::PathBuf,
    },
}
#[derive(clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}
fn main() {
    let args = Args::parse();

    match args.command {
        Commands::CSV { file, output } => {
            let mut wav = hound::WavReader::open(file).unwrap();
            println!("SampleRate: {}", wav.spec().sample_rate);
            println!("BitWidth: {}", wav.spec().bits_per_sample);
            println!("Channels: {}", wav.spec().channels);
            println!(
                "Duration: {}s ({} samples)",
                wav.duration() as f32 / wav.spec().sample_rate as f32,
                wav.duration()
            );

            let mut output = File::create(output).unwrap();

            let mut current_duration = std::time::Duration::from_nanos(0);
            let increment = std::time::Duration::from_nanos(
                (std::time::Duration::from_secs(1).as_nanos() / wav.spec().sample_rate as u128)
                    as u64,
            );
            for sample in wav.samples::<i16>().flat_map(|s| s.ok()) {
                output
                    .write_fmt(format_args!("{},{}\n", current_duration.as_nanos(), sample))
                    .unwrap();
                current_duration += increment;
            }
        }
        Commands::WAV { file, output } => {
            let file = File::open(file).unwrap();
            let content = read_to_string(file).unwrap();

            let spec = WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };

            let mut wav = hound::WavWriter::create(output, spec).unwrap();

            content.lines().for_each(|line| {
                let splits = line.split(",").skip(1).next().unwrap();
                let value = f32::from_str(splits).unwrap();
                wav.write_sample(value as i16).unwrap();
            });
            wav.flush().unwrap();
        }
        Commands::ToChunks { file, output } => {
            let mut wav = hound::WavReader::open(file).unwrap();
            println!("SampleRate: {}", wav.spec().sample_rate);
            println!("BitWidth: {}", wav.spec().bits_per_sample);
            println!("Channels: {}", wav.spec().channels);
            println!(
                "Duration: {}s ({} samples)",
                wav.duration() as f32 / wav.spec().sample_rate as f32,
                wav.duration()
            );

            let data = wav
                .samples::<i16>()
                .flat_map(|s| s.ok())
                .map(|sample| sample as f32)
                .collect::<Vec<_>>();
            let chunks = data
                .chunks(wav.spec().sample_rate as usize)
                .map(|chunk| {
                    let bytes: &[u8] = bytemuck::cast_slice(&chunk);
                    BASE64_STANDARD.encode(bytes)
                })
                .collect::<Vec<_>>();

            let mut current_duration = std::time::Duration::from_nanos(0);
            let increment = std::time::Duration::from_nanos(
                (std::time::Duration::from_secs(1).as_nanos() / wav.spec().sample_rate as u128)
                    as u64,
            ) * wav.spec().sample_rate;

            let mut output = File::create(output).unwrap();
            for sample in &chunks {
                output
                    .write_fmt(format_args!(
                        "{},{},{}\n",
                        current_duration.as_nanos(),
                        (current_duration + increment).as_nanos(),
                        sample
                    ))
                    .unwrap();
                current_duration += increment;
            }
        }
        Commands::FromChunks {
            file,
            output_prefix,
        } => {
            let file = File::open(file).unwrap();
            let content = read_to_string(file).unwrap();

            let samples = content
                .lines()
                .map(|line| {
                    let splits = line.split(",").skip(2).next().unwrap();
                    BASE64_STANDARD.decode(splits).unwrap()
                })
                .collect::<Vec<_>>();

            let base_path = if output_prefix.is_dir() {
                &output_prefix
            } else {
                output_prefix.parent().unwrap()
            };

            for (index, samples) in samples.iter().enumerate() {
                let samples = bytemuck::cast_slice::<u8, f32>(samples);
                let spec = WavSpec {
                    channels: 1,
                    sample_rate: 16000,
                    bits_per_sample: 16,
                    sample_format: SampleFormat::Int,
                };
                let name = format!(
                    "{}_{}.wav",
                    output_prefix.file_name().unwrap().to_str().unwrap(),
                    index
                );
                let mut wav = hound::WavWriter::create(base_path.join(name), spec).unwrap();

                for sample in samples {
                    wav.write_sample(*sample as i16).unwrap();
                }
                wav.flush().unwrap();
            }
        }
    }
}
