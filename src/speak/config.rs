//! Engine configuration and errors for the `speak` module.
//!
//! [`SpeakConfig`] describes one Kokoro bundle on disk plus the choices that
//! belong to a voice session: speaker, speed and text frontend. It keeps only
//! paths and plain values (no native handles), so it is cheap to clone and can
//! be built before the model is loaded.

use std::env;
use std::path::PathBuf;

/// Environment variable that overrides [`default_model_dir`].
pub const MODEL_DIR_ENV: &str = "COSH_KOKORO_MODEL_DIR";

/// Bundle name of the recommended Kokoro model: v1.1, Chinese + English,
/// 103 speakers. The v1.0 bundle (54 speakers) loads through the same code;
/// set [`SpeakConfig::model_dir`] to it.
pub const DEFAULT_MODEL_DIR_NAME: &str = "kokoro-multi-lang-v1_1";

/// Where the Kokoro bundle is expected when no explicit directory is given:
/// `$COSH_KOKORO_MODEL_DIR` when set, otherwise
/// `<user cache>/cosh/models/kokoro-multi-lang-v1_1`.
#[must_use]
pub fn default_model_dir() -> PathBuf {
    if let Some(dir) = env::var_os(MODEL_DIR_ENV).filter(|value| !value.is_empty()) {
        return PathBuf::from(dir);
    }
    let root = directories::BaseDirs::new()
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .unwrap_or_else(env::temp_dir);
    root.join("cosh")
        .join("models")
        .join(DEFAULT_MODEL_DIR_NAME)
}

/// Which text frontend turns written text into the phonemes Kokoro consumes.
///
/// sherpa-onnx 2.x no longer embeds espeak-ng: the built-in frontend is a
/// lexicon table, and anything else must be phonemized before generation.
/// `Kokoro` accepts pre-phonemized Unicode codepoints for exactly that case.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Phonemizer {
    /// sherpa-onnx's own lexicon frontend: it tokenizes with the
    /// `lexicon-*.txt` files shipped inside the model bundle (English and
    /// Chinese in both multilingual bundles) and needs nothing else.
    Lexicon,
    /// Phonemize outside sherpa-onnx with `piper-phonemize`, an espeak-ng
    /// based GPL-3.0-or-later crate, and hand Kokoro the codepoints.
    ///
    /// This is the path for Kokoro's other languages: their voices are
    /// present in `voices.bin` (Spanish, French, Hindi, Italian, Japanese,
    /// Brazilian Portuguese, ...) but sherpa-onnx ships no lexicon for them.
    /// Requires the `speak-phonemize` feature.
    #[cfg(feature = "speak-phonemize")]
    Piper {
        /// espeak-ng voice name, e.g. `"en-us"`, `"pt-br"`, `"fr"`.
        lang: String,
    },
}

/// One Kokoro TTS engine configuration.
///
/// [`Default`] points at [`default_model_dir`] with speaker 0, natural speed
/// and the lexicon frontend.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeakConfig {
    /// Directory holding the extracted bundle (`model.onnx`, `voices.bin`,
    /// `tokens.txt`, `lexicon-*.txt`).
    pub model_dir: PathBuf,
    /// Kokoro speaker id (`sid`). [`super::Speak::voices`] maps the ids of
    /// the loaded model to their names.
    pub voice: i32,
    /// Speech speed factor: `1.0` is the model's natural pace, larger is
    /// slower and smaller is faster. Must be greater than zero.
    pub speed: f32,
    /// ONNX Runtime inference threads. Two is a good default on a laptop;
    /// speech generation is bursty, so more threads rarely help.
    pub num_threads: i32,
    /// Lexicon files to use instead of the auto-discovered
    /// `<model_dir>/lexicon-*.txt`. Each path is validated at load time.
    pub lexicons: Vec<PathBuf>,
    /// Text normalization rule FSTs (comma-joined into sherpa-onnx's
    /// `rule_fsts`). The Chinese bundles ship `date-zh.fst`, `phone-zh.fst`
    /// and `number-zh.fst` for dates, phone numbers and digits; empty leaves
    /// normalization to the lexicon.
    pub rule_fsts: Vec<PathBuf>,
    /// Text frontend. See [`Phonemizer`].
    pub phonemizer: Phonemizer,
}

impl Default for SpeakConfig {
    fn default() -> Self {
        Self {
            model_dir: default_model_dir(),
            voice: 0,
            speed: 1.0,
            num_threads: 2,
            lexicons: Vec::new(),
            rule_fsts: Vec::new(),
            phonemizer: Phonemizer::Lexicon,
        }
    }
}

/// Everything `speak` can fail with.
///
/// The model errors carry the offending path so the message names what the
/// user has to fix instead of only which step failed.
#[derive(Debug, thiserror::Error)]
pub enum SpeakError {
    #[error(
        "Kokoro model directory `{0}` does not exist. Download a bundle from \
         https://github.com/k2-fsa/sherpa-onnx/releases/tag/tts-models and extract \
         it there, or point `{env}` at an existing extracted bundle",
        env = MODEL_DIR_ENV
    )]
    ModelDir(PathBuf),
    #[error(
        "`{0}` is missing from `{1}`. Expected a Kokoro bundle with \
         model.onnx (or model.int8.onnx), tokens.txt and voices.bin"
    )]
    ModelFile(String, PathBuf),
    #[error("required Kokoro file `{0}` does not exist")]
    MissingFile(PathBuf),
    #[error(
        "`{0}` contains no `lexicon-*.txt` and the lexicon frontend needs one. \
         Use a bundle that ships lexicons, list them in `lexicons`, or enable \
         the `speak-phonemize` feature and select `Phonemizer::Piper`"
    )]
    NoLexicon(PathBuf),
    #[error("path `{0}` is not valid UTF-8; sherpa-onnx takes file paths as UTF-8")]
    NonUtf8Path(PathBuf),
    #[error("sherpa-onnx could not load the Kokoro model in `{0}` (see its log above)")]
    Engine(PathBuf),
    #[error("voice id {sid} is out of range: this model has {speakers} speakers")]
    VoiceOutOfRange { sid: i32, speakers: u32 },
    #[error("speed must be greater than zero, got {0}")]
    BadSpeed(f32),
    #[error("nothing was synthesized: the text is empty or has no speakable content")]
    NoAudio,
    #[error("could not write the WAV file `{path}`")]
    Wav { path: PathBuf },
    #[error("no audio player played the generated audio (tried: {0})")]
    NoPlayer(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(feature = "speak-phonemize")]
    #[error("piper-phonemize could not phonemize the text for language `{lang}`")]
    Phonemize { lang: String },
}
