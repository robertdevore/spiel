//! Model loading policy and cache ownership, independent of UI and dictation entry points.
use crate::config::{Config, Paths};
use crate::error::{Result, SpielError};
use crate::{
    model,
    whisper::{self, Transcriber},
};
use std::sync::Mutex;

#[derive(Default)]
pub struct Transcription {
    cached: Mutex<Option<Transcriber>>,
}

impl Transcription {
    pub fn clear(&self) {
        *self.cached.lock().unwrap() = None;
    }

    fn load(&self, paths: &Paths, config: &Config) -> Result<Transcriber> {
        let spec = model::spec(&config.model).ok_or(SpielError::ModelMissing)?;
        let mut cached = self.cached.lock().unwrap();
        if let Some(model) = cached
            .as_ref()
            .filter(|model| model.model_id == config.model)
        {
            return Ok(model.clone());
        }
        let loaded = Transcriber::load(&paths.model_path(spec.filename), &config.model)?;
        *cached = Some(loaded.clone());
        Ok(loaded)
    }

    pub fn transcribe(&self, paths: &Paths, config: &Config, samples: &[f32]) -> Result<String> {
        if config.keep_model_loaded {
            return self.load(paths, config)?.transcribe(
                samples,
                &config.language,
                config.transcription_threads,
            );
        }
        self.clear();
        let spec = model::spec(&config.model).ok_or(SpielError::ModelMissing)?;
        whisper::transcribe_in_worker(
            &paths.model_path(spec.filename),
            &config.model,
            samples,
            &config.language,
            config.transcription_threads,
        )
    }

    pub fn warm_up(&self, paths: &Paths, config: &Config) -> Result<()> {
        let _ = self.load(paths, config)?;
        if !config.keep_model_loaded {
            self.clear();
        }
        Ok(())
    }
}
