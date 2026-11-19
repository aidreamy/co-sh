//! Local voice output: Kokoro text-to-speech through `sherpa-onnx`.
//!
//! The module owns one loaded Kokoro model and turns text into playable audio
//! without leaving the machine. Everything native lives in `sherpa-onnx`: this
//! module resolves the bundle files, validates the configuration, picks the
//! speaker and delegates synthesis. The only code written by hand is the small
//! amount sherpa-onnx's API does not cover — the stopwatch-free playback
//! handoff and the optional external phonemizer.
//!
//! # Enabling
//!
//! `speak` is behind the `speak` cargo feature because `sherpa-onnx` links a
//! static ONNX Runtime (`~100 MB`), which the default build deliberately
//! avoids. The optional `speak-phonemize` feature adds `piper-phonemize`
//! (espeak-ng); that crate is **GPL-3.0-or-later**, so it is off by default
//! and only the build that enables it takes on its obligations.
//!
//! ```toml
//! cosh = { version = "1", features = ["speak"] }
//! ```
//!
//! # Model
//!
//! Download the multilingual Kokoro bundle once and extract it where
//! [`default_model_dir`] looks (`<cache>/cosh/models/kokoro-multi-lang-v1_1`,
//! overridable with [`MODEL_DIR_ENV`]):
//!
//! ```text
//! curl -L -o kokoro.tar.bz2 \
//!   https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_1.tar.bz2
//! mkdir -p ~/.cache/cosh/models
//! tar xf kokoro.tar.bz2 -C ~/.cache/cosh/models
//! ```
//!
//! The directory must contain `model.onnx` (or the int8 `model.int8.onnx`),
//! `tokens.txt`, `voices.bin` and at least one `lexicon-*.txt`. The v1.0
//! bundle works the same way and is the one to use for its Brazilian
//! Portuguese, Spanish, French, Hindi, Italian and Japanese voices.
//!
//! # Languages
//!
//! sherpa-onnx 2.x dropped its embedded espeak-ng. Its own text frontend is
//! the lexicon table, which is why the multilingual bundles cover **Chinese
//! and English** — the only languages they ship lexicons for. Voices for the
//! other Kokoro languages are still present in `voices.bin`; driving them
//! needs the optional external phonemizer:
//!
//! ```text
//! SpeakConfig {
//!     phonemizer: Phonemizer::Piper { lang: "pt-br".into() },
//!     ..SpeakConfig::default()
//! }
//! ```
//!
//! # Example
//!
//! ```no_run
//! use cosh::speak::{Speak, SpeakConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let speak = Speak::load(SpeakConfig::default())?;
//! println!("{} speakers at {} Hz", speak.num_speakers(), speak.sample_rate());
//!
//! // Speak on the current thread (blocking) ...
//! speak.speak("Hello from cosh.")?;
//!
//! // ... or synthesize now, play or save the owned samples later.
//! let speech = speak.synthesize("This part is only synthesized.")?;
//! speech.save_wav("greeting.wav")?;
//! # Ok(())
//! # }
//! ```
//!
//! # Threading
//!
//! `Speak` is `Send + Sync` and one instance can serve several worker
//! threads; loading it is the expensive step, so keep it alive. `Speech` is
//! an owned `Vec<f32>` (sherpa-onnx's `GeneratedAudio` handle is neither
//! `Send` nor `Sync`), safe to hand to a playback thread. `Speech::play` and
//! `Speak::speak` block until the audio finishes, so call them off the TUI
//! event loop.

mod config;
mod engine;
#[cfg(feature = "speak-phonemize")]
mod phonemize;
mod player;
mod voice;

pub use config::{
    DEFAULT_MODEL_DIR_NAME, MODEL_DIR_ENV, Phonemizer, SpeakConfig, SpeakError, default_model_dir,
};
pub use engine::{Speak, Speech};
pub use voice::{Voice, find as find_voice};
