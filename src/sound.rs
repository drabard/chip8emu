use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioStatus, AudioDevice};
use sdl2::AudioSubsystem;

pub struct Sound {
    audio_device: AudioDevice<SquareWave>,
}

impl Sound {
    pub fn new(sdl_context: &sdl2::Sdl) -> Result<Sound, String> {
        let audio_subsystem = sdl_context.audio()?;
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None
        };
        let audio_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
            SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25
            }
        })?;

        let sound = Sound {
            audio_device: audio_device
        };

        Ok(sound)
    }

    pub fn play(self: &mut Self) {
        if self.audio_device.status() != AudioStatus::Playing {
            self.audio_device.resume();
        }
    }

    pub fn stop(self: &mut Self) {
        self.audio_device.pause();
    }
}

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

