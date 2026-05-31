use knoten_core_types::ast::Waveform;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{self, Sender};
use std::thread;

pub enum AudioCommand {
    PlaySound(String),
    LoopMusic(String),
    SetVolume(f32),
    PlayTone {
        channel: usize,
        freq: f32,
        duration_ms: u64,
        volume: f32,
        waveform: Waveform,
    },
    StopTone {
        channel: usize,
    },
}

pub struct AudioManager {
    tx: Sender<AudioCommand>,
}

impl AudioManager {
    pub fn new() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<AudioCommand>();

        thread::Builder::new()
            .name("AudioThread".to_string())
            .spawn(move || {
                let (_stream, stream_handle) = match OutputStream::try_default() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to init audio stream: {}", e);
                        return;
                    }
                };

                let mut sinks: HashMap<String, Sink> = HashMap::new();
                let mut synth_sinks: HashMap<usize, Sink> = HashMap::new();
                let mut global_volume = 1.0f32;

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        AudioCommand::PlaySound(path) => {
                            if let Ok(file) = File::open(&path)
                                && let Ok(decoder) = Decoder::new(BufReader::new(file))
                                && let Ok(sink) = Sink::try_new(&stream_handle)
                            {
                                sink.set_volume(global_volume);
                                sink.append(decoder);
                                sink.detach();
                            }
                        }
                        AudioCommand::LoopMusic(path) => {
                            if let Ok(file) = File::open(&path)
                                && let Ok(decoder) = Decoder::new(BufReader::new(file))
                                && let Ok(sink) = Sink::try_new(&stream_handle)
                            {
                                sink.set_volume(global_volume);
                                sink.append(decoder.repeat_infinite());
                                sinks.insert("bgm".to_string(), sink);
                            }
                        }
                        AudioCommand::SetVolume(vol) => {
                            global_volume = vol.clamp(0.0, 2.0);
                            for sink in sinks.values() {
                                sink.set_volume(global_volume);
                            }
                            for sink in synth_sinks.values() {
                                sink.set_volume(global_volume);
                            }
                        }
                        AudioCommand::PlayTone {
                            channel,
                            freq,
                            duration_ms,
                            volume,
                            waveform,
                        } => {
                            let sample_rate = 44100u32;
                            let num_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
                            let samples: Vec<f32> = (0..num_samples)
                                .map(|i| {
                                    let t = i as f32 / sample_rate as f32;
                                    generate_sample(t, freq, volume, waveform)
                                })
                                .collect();
                            let source = rodio::buffer::SamplesBuffer::new(1, sample_rate, samples);
                            if let Ok(sink) = rodio::Sink::try_new(&stream_handle) {
                                // Stop and replace any existing sink on this channel
                                if let Some(old) = synth_sinks.remove(&channel) {
                                    old.stop();
                                }
                                sink.set_volume(global_volume);
                                sink.append(source);
                                synth_sinks.insert(channel, sink);
                            }
                        }
                        AudioCommand::StopTone { channel } => {
                            if let Some(sink) = synth_sinks.remove(&channel) {
                                sink.stop();
                            }
                        }
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn audio thread: {}", e))?;

        Ok(Self { tx })
    }

    pub fn play_sound(&mut self, path: &str) -> Result<(), String> {
        let _ = self.tx.send(AudioCommand::PlaySound(path.to_string()));
        Ok(())
    }

    pub fn loop_music(&mut self, path: &str) -> Result<(), String> {
        let _ = self.tx.send(AudioCommand::LoopMusic(path.to_string()));
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) {
        let _ = self.tx.send(AudioCommand::SetVolume(volume));
    }

    pub fn play_tone(
        &mut self,
        channel: usize,
        freq: f32,
        duration_ms: u64,
        volume: f32,
        waveform: Waveform,
    ) {
        let _ = self.tx.send(AudioCommand::PlayTone {
            channel,
            freq,
            duration_ms,
            volume,
            waveform,
        });
    }

    pub fn stop_tone(&mut self, channel: usize) {
        let _ = self.tx.send(AudioCommand::StopTone { channel });
    }
}

fn generate_sample(t: f32, freq: f32, volume: f32, waveform: Waveform) -> f32 {
    match waveform {
        Waveform::Sine => (t * freq * 2.0 * std::f32::consts::PI).sin() * volume,
        Waveform::Square => {
            if (t * freq).fract() < 0.5 {
                volume
            } else {
                -volume
            }
        }
        Waveform::Sawtooth => 2.0 * (t * freq - (t * freq + 0.5).floor()) * volume,
        Waveform::Triangle => {
            let frac = t * freq - (t * freq).floor();
            ((2.0 * frac - 1.0).abs() * 2.0 - 1.0) * volume
        }
    }
}
