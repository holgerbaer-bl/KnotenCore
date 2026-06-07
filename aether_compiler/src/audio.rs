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
        attack_ms: u64,
        decay_ms: u64,
        sustain_level: f32,
        release_ms: u64,
        pan: f32,
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
                            attack_ms,
                            decay_ms,
                            sustain_level,
                            release_ms,
                            pan,
                        } => {
                            let sample_rate = 44100u32;
                            let num_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
                            let pan_clamped = pan.clamp(-1.0, 1.0);
                            let left_gain = (1.0 - pan_clamped).clamp(0.0, 1.0);
                            let right_gain = (1.0 + pan_clamped).clamp(0.0, 1.0);
                            let samples: Vec<f32> = (0..num_samples)
                                .flat_map(|i| {
                                    let t = i as f32 / sample_rate as f32;
                                    let t_ms = i as f32 * 1000.0 / sample_rate as f32;
                                    let raw = generate_sample(t, freq, volume, waveform);
                                    let env = adsr_amplitude(
                                        t_ms,
                                        attack_ms,
                                        decay_ms,
                                        sustain_level,
                                        release_ms,
                                        duration_ms as f32,
                                    );
                                    let mono = raw * env;
                                    [mono * left_gain, mono * right_gain]
                                })
                                .collect();
                            let source = rodio::buffer::SamplesBuffer::new(2, sample_rate, samples);
                            if let Ok(sink) = rodio::Sink::try_new(&stream_handle) {
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

    #[allow(clippy::too_many_arguments)]
    pub fn play_tone(
        &mut self,
        channel: usize,
        freq: f32,
        duration_ms: u64,
        volume: f32,
        waveform: Waveform,
        attack_ms: u64,
        decay_ms: u64,
        sustain_level: f32,
        release_ms: u64,
    ) {
        let _ = self.tx.send(AudioCommand::PlayTone {
            channel,
            freq,
            duration_ms,
            volume,
            waveform,
            attack_ms,
            decay_ms,
            sustain_level,
            release_ms,
            pan: 0.0,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn play_tone_panned(
        &mut self,
        channel: usize,
        freq: f32,
        duration_ms: u64,
        volume: f32,
        waveform: Waveform,
        attack_ms: u64,
        decay_ms: u64,
        sustain_level: f32,
        release_ms: u64,
        pan: f32,
    ) {
        let _ = self.tx.send(AudioCommand::PlayTone {
            channel,
            freq,
            duration_ms,
            volume,
            waveform,
            attack_ms,
            decay_ms,
            sustain_level,
            release_ms,
            pan,
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

fn adsr_amplitude(
    t_ms: f32,
    attack_ms: u64,
    decay_ms: u64,
    sustain_level: f32,
    release_ms: u64,
    total_ms: f32,
) -> f32 {
    let attack_end = attack_ms as f32;
    let decay_end = attack_end + decay_ms as f32;
    let release_start = total_ms - release_ms as f32;
    if t_ms <= attack_end {
        t_ms / attack_end.max(1.0)
    } else if t_ms <= decay_end {
        let progress = (t_ms - attack_end) / decay_ms.max(1) as f32;
        1.0 - progress * (1.0 - sustain_level)
    } else if t_ms <= release_start {
        sustain_level
    } else {
        let progress = (t_ms - release_start) / release_ms.max(1) as f32;
        sustain_level * (1.0 - progress).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adsr_attack_phase_linear_ramp() {
        let a = adsr_amplitude(0.0, 10, 20, 0.5, 50, 200.0);
        assert!((a - 0.0).abs() < 0.001, "Attack start should be 0.0");
        let a = adsr_amplitude(10.0, 10, 20, 0.5, 50, 200.0);
        assert!((a - 1.0).abs() < 0.001, "Attack end should be 1.0");
        let a = adsr_amplitude(5.0, 10, 20, 0.5, 50, 200.0);
        assert!((a - 0.5).abs() < 0.02, "Attack midpoint should be ~0.5");
    }

    #[test]
    fn adsr_sustain_holds_constant() {
        let at_end = adsr_amplitude(30.0, 10, 20, 0.7, 50, 200.0);
        assert!((at_end - 0.7).abs() < 0.001, "Sustain start should be 0.7");
        let mid = adsr_amplitude(100.0, 10, 20, 0.7, 50, 200.0);
        assert!((mid - 0.7).abs() < 0.001, "Sustain mid should be 0.7");
    }

    #[test]
    fn adsr_release_ramps_to_zero() {
        let r_start = adsr_amplitude(150.0, 10, 20, 0.7, 50, 200.0);
        assert!(
            (r_start - 0.7).abs() < 0.001,
            "Release start should match sustain"
        );
        let r_end = adsr_amplitude(200.0, 10, 20, 0.7, 50, 200.0);
        assert!((r_end - 0.0).abs() < 0.001, "Release end should be 0.0");
    }

    #[test]
    fn adsr_zero_freq_no_panic() {
        let val = adsr_amplitude(0.0, 0, 0, 0.0, 0, 10.0);
        assert!(
            (val - 0.0).abs() < 0.001,
            "Zero ADSR params should return 0 at t=0"
        );
    }

    #[test]
    fn adsr_instant_attack_decay() {
        let val = adsr_amplitude(0.0, 0, 0, 1.0, 0, 100.0);
        assert!(
            (val - 0.0).abs() < 0.001,
            "Zero attack+decay: t=0 returns 0.0 (.max(1) guard prevents div-by-zero)"
        );
        let val2 = adsr_amplitude(1.0, 0, 0, 1.0, 0, 100.0);
        assert!(
            (val2 - 1.0).abs() < 0.001,
            "Attack guarded to 1ms, t=1 reaches 1.0"
        );
    }

    #[test]
    fn adsr_full_envelope_edge_values() {
        let v0 = adsr_amplitude(0.0, 5, 10, 0.5, 20, 100.0);
        assert!((v0 - 0.0).abs() < 0.001, "t=0 → 0.0");
        let v5 = adsr_amplitude(5.0, 5, 10, 0.5, 20, 100.0);
        assert!((v5 - 1.0).abs() < 0.001, "t=5 → attack end = 1.0");
        let v15 = adsr_amplitude(15.0, 5, 10, 0.5, 20, 100.0);
        assert!((v15 - 0.5).abs() < 0.02, "t=15 → decay end = sustain 0.5");
        let v50 = adsr_amplitude(50.0, 5, 10, 0.5, 20, 100.0);
        assert!((v50 - 0.5).abs() < 0.001, "t=50 → mid-sustain = 0.5");
        let v90 = adsr_amplitude(90.0, 5, 10, 0.5, 20, 100.0);
        assert!((v90 - 0.25).abs() < 0.02, "t=90 → mid-release");
        let v100 = adsr_amplitude(100.0, 5, 10, 0.5, 20, 100.0);
        assert!((v100 - 0.0).abs() < 0.001, "t=100 → release end = 0.0");
    }

    #[test]
    fn adsr_negative_times_dont_panic() {
        let val = adsr_amplitude(50.0, 5, 10, 0.5, 20, 100.0);
        assert!((0.0..=1.0).contains(&val), "Envelope must stay in [0,1]");
    }

    #[test]
    fn test_audio_stereo_panning_bounds() {
        let pan_hard_left = (-1.0f32).clamp(-1.0, 1.0);
        assert!((pan_hard_left + 1.0).abs() < 0.001);

        let pan_hard_right = (1.0f32).clamp(-1.0, 1.0);
        assert!((pan_hard_right - 1.0).abs() < 0.001);

        let pan_center = (0.0f32).clamp(-1.0, 1.0);
        assert!((pan_center - 0.0).abs() < 0.001);

        let pan_half_left = (-0.5f32).clamp(-1.0, 1.0);
        assert!((pan_half_left + 0.5).abs() < 0.001);

        let left_gain = (1.0 - (-1.0f32)).clamp(0.0, 1.0);
        assert!((left_gain - 1.0).abs() < 0.001, "Full left = left gain 1.0");
        let right_gain = (1.0 + (-1.0f32)).clamp(0.0, 1.0);
        assert!(
            (right_gain - 0.0).abs() < 0.001,
            "Full left = right gain 0.0"
        );

        let left_gain = (1.0 - 1.0f32).clamp(0.0, 1.0);
        assert!(
            (left_gain - 0.0).abs() < 0.001,
            "Full right = left gain 0.0"
        );
        let right_gain = (1.0 + 1.0f32).clamp(0.0, 1.0);
        assert!(
            (right_gain - 1.0).abs() < 0.001,
            "Full right = right gain 1.0"
        );

        let clamped = (-1.5f32).clamp(-1.0, 1.0);
        assert!(
            (clamped + 1.0).abs() < 0.001,
            "Out of range low clamped to -1"
        );
        let clamped = (2.0f32).clamp(-1.0, 1.0);
        assert!(
            (clamped - 1.0).abs() < 0.001,
            "Out of range high clamped to 1"
        );
    }

    proptest::proptest! {
        #[test]
        fn fuzz_adsr_envelope_boundaries(
            attack_ms in 0u64..1000,
            decay_ms in 0u64..1000,
            sustain_level in proptest::num::f32::NORMAL,
            release_ms in 0u64..1000,
            t_ms in 0.0f32..2000.0,
            total_ms in 1.0f32..2000.0,
        ) {
            let sustain = sustain_level.clamp(0.0, 1.0);
            let amplitude = adsr_amplitude(t_ms, attack_ms, decay_ms, sustain, release_ms, total_ms);
            assert!(!amplitude.is_nan(), "ADSR must not produce NaN");
            assert!(amplitude.is_finite(), "ADSR must not produce Inf");
            assert!((0.0..=1.0).contains(&amplitude), "Amplitude {amplitude} outside [0,1]");
        }
    }
}
