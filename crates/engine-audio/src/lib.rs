use engine_core::{EngineError, Result};
use kira::{
    manager::{AudioManager, AudioManagerSettings, DefaultBackend},
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct AudioClip {
    data: StaticSoundData,
    path: PathBuf,
}

impl AudioClip {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct AudioModule {
    manager: Option<AudioManager<DefaultBackend>>,
    init_error: Option<String>,
    pub master_volume: f64,
    pub sfx_volume: f64,
    pub music_volume: f64,
    music_active: bool,
}

impl Default for AudioModule {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioModule {
    pub fn new() -> Self {
        Self {
            manager: None,
            init_error: None,
            master_volume: 1.0,
            sfx_volume: 1.0,
            music_volume: 0.7,
            music_active: false,
        }
    }

    pub fn update(&mut self) -> Result<()> {
        log::trace!(
            target: "engine::audio",
            "Audio update completed (music_playing={})",
            self.music_active
        );
        Ok(())
    }

    pub fn load_clip(&self, path: impl AsRef<Path>) -> Result<AudioClip> {
        let path = path.as_ref();
        let data = StaticSoundData::from_file(path).map_err(|error| EngineError::AssetLoad {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;

        Ok(AudioClip {
            data,
            path: path.to_path_buf(),
        })
    }

    pub fn load_music_clip_with_fallback(&self, stem_path: impl AsRef<Path>) -> Result<AudioClip> {
        let stem_path = stem_path.as_ref();
        let mut reasons = Vec::new();

        for candidate in music_fallback_candidates(stem_path) {
            match self.load_clip(&candidate) {
                Ok(clip) => return Ok(clip),
                Err(error) => reasons.push(format!("{} ({})", candidate.display(), error)),
            }
        }

        Err(EngineError::Audio(format!(
            "failed to load music clip using fallback order OGG->WAV->MP3: {}",
            reasons.join("; ")
        )))
    }

    pub fn play(&mut self, clip: &AudioClip) -> Result<StaticSoundHandle> {
        self.ensure_initialized()?;

        let Some(manager) = self.manager.as_mut() else {
            return Err(EngineError::Audio(
                "audio backend unavailable after initialization attempt".to_owned(),
            ));
        };

        manager
            .play(clip.data.volume(self.master_volume * self.sfx_volume))
            .map_err(|error| EngineError::Audio(error.to_string()))
    }

    pub fn play_music(&mut self, clip: &AudioClip) -> Result<StaticSoundHandle> {
        self.ensure_initialized()?;

        let Some(manager) = self.manager.as_mut() else {
            return Err(EngineError::Audio(
                "audio backend unavailable after initialization attempt".to_owned(),
            ));
        };

        let handle = manager
            .play(
                clip.data
                    .volume(self.master_volume * self.music_volume)
                    .loop_region(..),
            )
            .map_err(|error| EngineError::Audio(error.to_string()))?;

        self.music_active = true;
        Ok(handle)
    }

    pub fn play_music_with_fallback(
        &mut self,
        stem_path: impl AsRef<Path>,
    ) -> Result<StaticSoundHandle> {
        let clip = self.load_music_clip_with_fallback(stem_path)?;
        self.play_music(&clip)
    }

    pub fn backend_type_names(&self) -> (&'static str, &'static str) {
        (
            std::any::type_name::<cpal::SampleRate>(),
            std::any::type_name::<kira::manager::AudioManagerSettings<kira::manager::DefaultBackend>>(
            ),
        )
    }

    fn ensure_initialized(&mut self) -> Result<()> {
        if self.manager.is_some() {
            return Ok(());
        }

        if let Some(error) = self.init_error.as_ref() {
            return Err(EngineError::Audio(format!(
                "audio backend unavailable (cached): {}",
                error
            )));
        }

        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(manager) => {
                self.manager = Some(manager);
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                self.init_error = Some(message.clone());
                log::warn!(
                    target: "engine::audio",
                    "Audio backend initialization failed; continuing without audio: {}",
                    message
                );
                Err(EngineError::Audio(message))
            }
        }
    }
}

pub fn module_name() -> &'static str {
    "engine-audio"
}

fn music_fallback_candidates(stem_path: &Path) -> [PathBuf; 3] {
    [
        stem_path.with_extension("ogg"),
        stem_path.with_extension("wav"),
        stem_path.with_extension("mp3"),
    ]
}

#[cfg(test)]
mod tests;
