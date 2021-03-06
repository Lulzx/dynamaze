use std::collections::HashMap;
use std::sync::Mutex;

use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, GainNode, HtmlAudioElement};

use crate::options;

const MUSIC_VOLUME: f32 = 0.6;
const SOUND_VOLUME: f32 = 0.4;

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum Music {
    Menu,
    InGame,
}

fn calc_gain(global_scale: f32, options_level: u8) -> f32 {
    global_scale * (f32::from(options_level)) / 100.0
}

fn ramp_gain(gain: web_sys::AudioParam, value: f32) {
    gain.exponential_ramp_to_value_at_time(value, 0.01).unwrap_throw();
}

impl Music {
    fn load(self) -> HtmlAudioElement {
        let path = match self {
            Music::Menu => "assets/BlueEther.mp3",
            Music::InGame => "assets/ElectricSweater.mp3",
        };

        let result = HtmlAudioElement::new_with_src(path).unwrap_throw();
        result.set_loop(true);
        result
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum Sound {
    YourTurn,
}

impl Sound {
    fn load(self) -> HtmlAudioElement {
        let path = match self {
            Sound::YourTurn => "assets/TurnPing.wav",
        };

        HtmlAudioElement::new_with_src(path).unwrap_throw()
    }
}

pub struct SoundEngine {
    context: AudioContext,
    music_sources: Mutex<HashMap<Music, HtmlAudioElement>>,
    sound_sources: Mutex<HashMap<Sound, HtmlAudioElement>>,
    music_gain: GainNode,
    sound_gain: GainNode,
    current_music: Mutex<Option<Music>>,
}

impl SoundEngine {
    pub fn new() -> SoundEngine {
        let context = AudioContext::new().unwrap_throw();
        let music_gain = context
            .create_gain()
            .expect_throw("Failed to create music gain node");
        music_gain
            .gain()
            .set_value(calc_gain(MUSIC_VOLUME, options::HANDLE.fetch().music_level));
        music_gain
            .connect_with_audio_node(&context.destination())
            .unwrap_throw();
        let sound_gain = context
            .create_gain()
            .expect_throw("Failed to create sound  gain node");
        sound_gain
            .gain()
            .set_value(calc_gain(SOUND_VOLUME, options::HANDLE.fetch().sound_level));
        sound_gain
            .connect_with_audio_node(&context.destination())
            .unwrap_throw();
        SoundEngine {
            context,
            music_sources: Mutex::new(HashMap::new()),
            sound_sources: Mutex::new(HashMap::new()),
            music_gain,
            sound_gain,
            current_music: Mutex::new(None),
        }
    }

    pub fn unpause(&self) {
        if let web_sys::AudioContextState::Suspended = self.context.state() {
            let _ = self.context.resume();
            let music = {
                let mut current_music = self.current_music.lock().unwrap();
                current_music.take()
            };
            if let Some(music) = music {
                self.play_music(music);
            }
        }
    }

    pub fn play_music(&self, music: Music) {
        let mut current_music = self.current_music.lock().unwrap();
        if *current_music == Some(music) {
            return;
        }
        let mut music_sources = self.music_sources.lock().unwrap();
        if let Some(ref old_music) = *current_music {
            if let Some(old_source) = music_sources.get(old_music) {
                old_source.pause().unwrap_throw();
            }
        }
        let source = music_sources.entry(music).or_insert_with(|| {
            let source = music.load();
            let source_node = self
                .context
                .create_media_element_source(&source)
                .unwrap_throw();
            source_node
                .connect_with_audio_node(&self.music_gain)
                .unwrap_throw();
            source
        });
        let _ = source.play().unwrap_throw();
        *current_music = Some(music);
    }

    pub fn play_sound(&self, snd: Sound) {
        let _ = self.context.resume();
        let mut sound_sources = self.sound_sources.lock().unwrap();
        let source = sound_sources.entry(snd).or_insert_with(|| {
            let source = snd.load();
            let source_node = self
                .context
                .create_media_element_source(&source)
                .unwrap_throw();
            source_node
                .connect_with_audio_node(&self.sound_gain)
                .unwrap_throw();
            source
        });
        let _ = source.play().unwrap_throw();
    }

    pub fn fetch_volume(&self) {
        self.poke_options(&*options::HANDLE.fetch());
    }

    pub fn poke_options(&self, new_options: &options::GameOptions) {
        ramp_gain(self.music_gain.gain(), calc_gain(MUSIC_VOLUME, new_options.music_level));
        ramp_gain(self.sound_gain.gain(), calc_gain(SOUND_VOLUME, new_options.sound_level));
    }
}

impl Default for SoundEngine {
    fn default() -> Self {
        Self::new()
    }
}
