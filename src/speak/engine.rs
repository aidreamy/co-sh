//! The synthesis engine: one loaded Kokoro model, shared by its callers.
//!
//! `sherpa_onnx::OfflineTts` is `Send + Sync`, so one [`Speak`] can serve the
//! TUI's worker threads. `GeneratedAudio` is neither, so [`Speech`] copies the
//! samples into an owned `Vec<f32>` before returning: the caller gets plain,
//! movable data, not a native handle.

use std::path::{Path, PathBuf};
use std::time::Duration;

use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKokoroModelConfig,
    OfflineTtsModelConfig,
};

use super::config::{Phonemizer, SpeakConfig, SpeakError};
use super::player;
use super::voice::{self, Voice};

/// A Kokoro TTS engine bound to one bundle and one voice configuration.
///
/// Loading is the expensive step (the model is ~310 MB), so build one and keep
/// it; synthesis calls only touch the loaded engine. Change the speaker or
/// speed on the instance with [`Speak::set_voice`] and [`Speak::set_speed`]
/// instead of reloading.
pub struct Speak {
    tts: OfflineTts,
    config: SpeakConfig,
    lexicons: Vec<PathBuf>,
}

impl Speak {
    /// Resolve the bundle files and load the model.
    ///
    /// All files are checked before sherpa-onnx is called, so a missing piece
    /// reports its own path instead of a generic engine failure.
    pub fn load(config: SpeakConfig) -> Result<Self, SpeakError> {
        if !(config.speed > 0.0 && config.speed.is_finite()) {
            return Err(SpeakError::BadSpeed(config.speed));
        }
        let files = ModelFiles::resolve(&config)?;

        let mut sherpa_config = OfflineTtsConfig {
            model: OfflineTtsModelConfig {
                kokoro: OfflineTtsKokoroModelConfig {
                    model: Some(files.model),
                    voices: Some(files.voices),
                    tokens: Some(files.tokens),
                    lexicon: files.lexicon,
                    // The per-request `speed` already applies the pace;
                    // leaving the model-level scale at its neutral value
                    // keeps the two from multiplying.
                    length_scale: 1.0,
                    ..Default::default()
                },
                num_threads: config.num_threads.max(1),
                ..Default::default()
            },
            rule_fsts: files.rule_fsts,
            // Kokoro ignores anything but 1 and logs a warning otherwise:
            // it batch-splits sentences itself, one sentence per Process call.
            max_num_sentences: 1,
            ..Default::default()
        };
        sherpa_config.silence_scale = 1.0;

        let tts = OfflineTts::create(&sherpa_config)
            .ok_or_else(|| SpeakError::Engine(config.model_dir.clone()))?;
        let speakers = tts.num_speakers().max(1) as u32;
        if config.voice < 0 || config.voice as u32 >= speakers {
            return Err(SpeakError::VoiceOutOfRange {
                sid: config.voice,
                speakers,
            });
        }

        Ok(Self {
            tts,
            lexicons: files.lexicon_paths,
            config,
        })
    }

    /// The configuration this engine was loaded with.
    #[must_use]
    pub fn config(&self) -> &SpeakConfig {
        &self.config
    }

    /// Output sample rate in Hz (24000 for both multilingual bundles).
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.tts.sample_rate().max(0) as u32
    }

    /// Number of speakers the loaded model reports.
    #[must_use]
    pub fn num_speakers(&self) -> u32 {
        self.tts.num_speakers().max(1) as u32
    }

    /// Language metadata recorded in the model, e.g. `"zh en-us"`.
    /// Empty when the model carries none.
    #[must_use]
    pub fn lang(&self) -> String {
        self.tts.lang()
    }

    /// The lexicon files the engine is tokenizing with (empty for the
    /// external phonemizer path).
    #[must_use]
    pub fn lexicons(&self) -> &[PathBuf] {
        &self.lexicons
    }

    /// Names for the loaded model's speakers; empty when the bundle is not a
    /// known Kokoro release (the ids remain usable).
    #[must_use]
    pub fn voices(&self) -> &'static [Voice] {
        voice::catalog(self.num_speakers())
    }

    /// Look up one of this model's voices by name, e.g. `"af_maple"`.
    #[must_use]
    pub fn voice(&self, name: &str) -> Option<Voice> {
        self.voices().iter().copied().find(|v| v.name == name)
    }

    /// Select a speaker for later synthesis calls.
    pub fn set_voice(&mut self, sid: i32) -> Result<(), SpeakError> {
        if sid < 0 || sid as u32 >= self.num_speakers() {
            return Err(SpeakError::VoiceOutOfRange {
                sid,
                speakers: self.num_speakers(),
            });
        }
        self.config.voice = sid;
        Ok(())
    }

    /// Set the speech pace for later synthesis calls (`> 0`; `1.0` natural).
    pub fn set_speed(&mut self, speed: f32) -> Result<(), SpeakError> {
        if !(speed > 0.0 && speed.is_finite()) {
            return Err(SpeakError::BadSpeed(speed));
        }
        self.config.speed = speed;
        Ok(())
    }

    /// Synthesize `text` into one owned buffer.
    pub fn synthesize(&self, text: &str) -> Result<Speech, SpeakError> {
        self.synthesize_streaming(text, |_, _| true)
    }

    /// Synthesize `text`, handing each generated chunk to `on_chunk`.
    ///
    /// sherpa-onnx splits the text into sentences, runs them one at a time and
    /// calls back with that sentence's samples and the fraction finished
    /// (`0.0..=1.0`) — a player can start speaking before the last sentence is
    /// generated. Returning `false` stops generation early and keeps only what
    /// was produced so far. The callback sees each chunk once, not the
    /// accumulated audio.
    ///
    /// The callback must be `'static` because sherpa-onnx's C callback
    /// outlives the borrow; send chunks through a channel if they have to
    /// reach a caller-owned sink.
    pub fn synthesize_streaming<F>(&self, text: &str, on_chunk: F) -> Result<Speech, SpeakError>
    where
        F: FnMut(&[f32], f32) -> bool + 'static,
    {
        if text.trim().is_empty() {
            return Err(SpeakError::NoAudio);
        }

        let (text, generation) = self.prepare(text)?;

        let audio = self
            .tts
            .generate_with_config(&text, &generation, Some(on_chunk))
            .ok_or(SpeakError::NoAudio)?;
        let speech = Speech {
            samples: audio.samples().to_vec(),
            sample_rate: audio.sample_rate().max(0) as u32,
        };
        if speech.samples.is_empty() {
            return Err(SpeakError::NoAudio);
        }
        Ok(speech)
    }

    /// Build the per-request options, moving the text through the external
    /// phonemizer first when one is selected.
    #[cfg(feature = "speak-phonemize")]
    fn prepare(&self, text: &str) -> Result<(String, GenerationConfig), SpeakError> {
        match &self.config.phonemizer {
            Phonemizer::Lexicon => Ok((text.to_string(), self.generation(None))),
            Phonemizer::Piper { lang } => {
                let codepoints = Some(super::phonemize::codepoints(text, lang)?);
                // The text was consumed by the phonemizer; sherpa-onnx reads
                // the codepoints instead.
                Ok((String::new(), self.generation(codepoints)))
            }
        }
    }

    #[cfg(not(feature = "speak-phonemize"))]
    fn prepare(&self, text: &str) -> Result<(String, GenerationConfig), SpeakError> {
        Ok((text.to_string(), self.generation(None)))
    }

    fn generation(&self, codepoints: Option<Vec<Vec<i32>>>) -> GenerationConfig {
        GenerationConfig {
            sid: self.config.voice,
            speed: self.config.speed,
            silence_scale: 1.0,
            phoneme_codepoints: codepoints,
            ..Default::default()
        }
    }

    /// Synthesize `text` and play it through the system audio player,
    /// blocking until playback finishes.
    ///
    /// Call this from a worker thread: generation and playback together take
    /// roughly the duration of the speech plus the synthesis time.
    pub fn speak(&self, text: &str) -> Result<Speech, SpeakError> {
        let speech = self.synthesize(text)?;
        speech.play()?;
        Ok(speech)
    }
}

/// One synthesized utterance, owned and movable across threads.
#[derive(Debug, Clone, PartialEq)]
pub struct Speech {
    samples: Vec<f32>,
    sample_rate: u32,
}

impl Speech {
    /// Normalized `f32` PCM samples (mono).
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Output sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Length of the utterance.
    #[must_use]
    pub fn duration(&self) -> Duration {
        if self.sample_rate == 0 {
            return Duration::ZERO;
        }
        Duration::from_secs_f64(self.samples.len() as f64 / self.sample_rate as f64)
    }

    /// Consume the utterance and take the samples.
    #[must_use]
    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    /// Write the utterance as a WAV file with sherpa-onnx's own writer.
    pub fn save_wav(&self, path: impl AsRef<Path>) -> Result<(), SpeakError> {
        let path = path.as_ref();
        let name = path
            .to_str()
            .ok_or_else(|| SpeakError::NonUtf8Path(path.to_path_buf()))?;
        if sherpa_onnx::write(name, &self.samples, self.sample_rate as i32) {
            Ok(())
        } else {
            Err(SpeakError::Wav {
                path: path.to_path_buf(),
            })
        }
    }

    /// Play the utterance through the system player, blocking until done.
    pub fn play(&self) -> Result<(), SpeakError> {
        player::play(self)
    }
}

/// The bundle files, resolved and validated before the model is loaded.
#[derive(Debug)]
struct ModelFiles {
    model: String,
    voices: String,
    tokens: String,
    lexicon: Option<String>,
    lexicon_paths: Vec<PathBuf>,
    rule_fsts: Option<String>,
}

impl ModelFiles {
    fn resolve(config: &SpeakConfig) -> Result<Self, SpeakError> {
        let dir = &config.model_dir;
        if !dir.is_dir() {
            return Err(SpeakError::ModelDir(dir.clone()));
        }

        // Both releases ship `model.onnx`; the int8 tarball ships
        // `model.int8.onnx`. Accept either without an extra setting.
        let model = ["model.onnx", "model.int8.onnx"]
            .iter()
            .map(|name| dir.join(name))
            .find(|path| path.is_file())
            .ok_or_else(|| {
                SpeakError::ModelFile("model.onnx or model.int8.onnx".into(), dir.clone())
            })?;
        let tokens = required(dir, "tokens.txt")?;
        let voices = required(dir, "voices.bin")?;

        let lexicons: Vec<PathBuf> = if config.lexicons.is_empty() {
            discovered_lexicons(dir)?
        } else {
            for path in &config.lexicons {
                if !path.is_file() {
                    return Err(SpeakError::MissingFile(path.clone()));
                }
            }
            config.lexicons.clone()
        };
        if matches!(config.phonemizer, Phonemizer::Lexicon) && lexicons.is_empty() {
            return Err(SpeakError::NoLexicon(dir.clone()));
        }

        let mut rule_fsts = config.rule_fsts.clone();
        rule_fsts.retain(|path| path.is_file());

        Ok(Self {
            model: utf8(&model)?,
            tokens: utf8(&tokens)?,
            voices: utf8(&voices)?,
            lexicon: join_utf8(&lexicons)?,
            lexicon_paths: lexicons,
            rule_fsts: join_utf8(&rule_fsts)?,
        })
    }
}

fn required(dir: &Path, name: &str) -> Result<PathBuf, SpeakError> {
    let path = dir.join(name);
    if path.is_file() {
        Ok(path)
    } else {
        Err(SpeakError::MissingFile(path))
    }
}

/// Every `lexicon-*.txt` in the bundle, in stable order.
///
/// The official bundles ship `lexicon-us-en.txt`, `lexicon-gb-en.txt` and
/// `lexicon-zh.txt`; sherpa-onnx merges them into one lookup table, so the
/// order only decides which entry wins on a collision (the later file).
/// Sorted order puts `us-en` after `gb-en`, matching the upstream English
/// examples.
fn discovered_lexicons(dir: &Path) -> Result<Vec<PathBuf>, SpeakError> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("lexicon-") && name.ends_with(".txt"))
        })
        .collect();
    found.sort();
    Ok(found)
}

fn utf8(path: &Path) -> Result<String, SpeakError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| SpeakError::NonUtf8Path(path.to_path_buf()))
}

fn join_utf8(paths: &[PathBuf]) -> Result<Option<String>, SpeakError> {
    if paths.is_empty() {
        return Ok(None);
    }
    let mut joined = Vec::with_capacity(paths.len());
    for path in paths {
        joined.push(utf8(path)?);
    }
    Ok(Some(joined.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speak::config::Phonemizer;

    /// A directory shaped like an extracted bundle, without the huge model.
    fn bundle(lexicons: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in ["model.onnx", "tokens.txt", "voices.bin"] {
            std::fs::write(dir.path().join(name), b"").unwrap();
        }
        for name in lexicons {
            std::fs::write(dir.path().join(name), b"").unwrap();
        }
        dir
    }

    fn config(dir: &Path) -> SpeakConfig {
        SpeakConfig {
            model_dir: dir.to_path_buf(),
            ..SpeakConfig::default()
        }
    }

    #[test]
    fn a_missing_directory_is_named_in_the_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-downloaded");
        let error = ModelFiles::resolve(&config(&missing)).unwrap_err();
        assert!(matches!(error, SpeakError::ModelDir(ref path) if path == &missing));
        let message = error.to_string();
        assert!(message.contains("releases/tag/tts-models"));
        assert!(message.contains(crate::speak::config::MODEL_DIR_ENV));
    }

    #[test]
    fn lexicons_are_discovered_sorted_and_comma_joined() {
        let dir = bundle(&[
            "lexicon-zh.txt",
            "lexicon-us-en.txt",
            "lexicon-gb-en.txt",
            "notes.txt",
        ]);
        let files = ModelFiles::resolve(&config(dir.path())).unwrap();
        assert_eq!(files.lexicon_paths.len(), 3);
        let joined = [
            dir.path().join("lexicon-gb-en.txt"),
            dir.path().join("lexicon-us-en.txt"),
            dir.path().join("lexicon-zh.txt"),
        ]
        .iter()
        .map(|path| path.to_str().unwrap())
        .collect::<Vec<_>>()
        .join(",");
        assert_eq!(files.lexicon.as_deref(), Some(joined.as_str()));
    }

    #[test]
    fn the_lexicon_frontend_requires_a_lexicon() {
        let dir = bundle(&[]);
        let error = ModelFiles::resolve(&config(dir.path())).unwrap_err();
        assert!(matches!(error, SpeakError::NoLexicon(ref path) if path == dir.path()));
        assert!(error.to_string().contains("speak-phonemize"));
    }

    #[test]
    fn an_explicit_lexicon_must_exist() {
        let dir = bundle(&[]);
        let mut config = config(dir.path());
        config.lexicons = vec![dir.path().join("lexicon-missing.txt")];
        let error = ModelFiles::resolve(&config).unwrap_err();
        assert!(matches!(error, SpeakError::MissingFile(_)));
    }

    #[test]
    fn the_int8_checkpoint_is_accepted() {
        let dir = bundle(&["lexicon-us-en.txt"]);
        std::fs::remove_file(dir.path().join("model.onnx")).unwrap();
        std::fs::write(dir.path().join("model.int8.onnx"), b"").unwrap();
        let files = ModelFiles::resolve(&config(dir.path())).unwrap();
        assert!(files.model.ends_with("model.int8.onnx"));
    }

    #[test]
    fn the_phonemizer_path_needs_no_lexicon() {
        let dir = bundle(&[]);
        let mut config = config(dir.path());
        config.phonemizer = Phonemizer::Lexicon;
        assert!(ModelFiles::resolve(&config).is_err());
        #[cfg(feature = "speak-phonemize")]
        {
            config.phonemizer = Phonemizer::Piper {
                lang: "pt-br".into(),
            };
            let files = ModelFiles::resolve(&config).unwrap();
            assert!(files.lexicon.is_none());
        }
    }
}
