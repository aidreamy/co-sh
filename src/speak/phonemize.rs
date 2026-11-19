//! The external phonemizer used by [`Phonemizer::Piper`].
//!
//! sherpa-onnx 2.x removed espeak-ng, so a Kokoro language without a lexicon
//! has to be phonemized before generation and passed as Unicode codepoints.
//! `piper-phonemize` does that with espeak-ng and embeds its own
//! `espeak-ng-data`, so no external data directory is needed.
//!
//! [`Phonemizer::Piper`]: super::config::Phonemizer::Piper

use super::config::SpeakError;

/// Phonemize `text` into one codepoint vector per sentence, the shape
/// `GenerationConfig::phoneme_codepoints` expects.
pub(crate) fn codepoints(text: &str, lang: &str) -> Result<Vec<Vec<i32>>, SpeakError> {
    let result = piper_phonemize::phonemize(text, lang).ok_or_else(|| SpeakError::Phonemize {
        lang: lang.to_string(),
    })?;
    let sentences: Vec<Vec<i32>> = (0..result.num_sentences())
        .map(|index| result.get_phonemes(index).unwrap_or_default())
        .filter(|phonemes| !phonemes.is_empty())
        .collect();
    if sentences.is_empty() {
        return Err(SpeakError::NoAudio);
    }
    Ok(sentences)
}
