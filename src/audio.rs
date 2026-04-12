use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};
use std::thread;

pub enum AudioCommand {
    PlaySound(String),
    LoopMusic(String),
    SetVolume(f32),
}

pub struct AudioManager {
    tx: Sender<AudioCommand>,
}

impl AudioManager {
    pub fn new() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<AudioCommand>();

        thread::Builder::new().name("AudioThread".to_string()).spawn(move || {
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to init audio stream: {}", e);
                    return;
                }
            };
            
            let mut sinks: HashMap<String, Sink> = HashMap::new();
            let mut global_volume = 1.0f32;

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::PlaySound(path) => {
                        if let Ok(file) = File::open(&path)
                            && let Ok(decoder) = Decoder::new(BufReader::new(file))
                                && let Ok(sink) = Sink::try_new(&stream_handle) {
                                    sink.set_volume(global_volume);
                                    sink.append(decoder);
                                    sink.detach();
                                }
                    }
                    AudioCommand::LoopMusic(path) => {
                        if let Ok(file) = File::open(&path)
                            && let Ok(decoder) = Decoder::new(BufReader::new(file))
                                && let Ok(sink) = Sink::try_new(&stream_handle) {
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
                    }
                }
            }
        }).map_err(|e| format!("Failed to spawn audio thread: {}", e))?;

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
}
