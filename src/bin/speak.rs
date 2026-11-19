//! Manual voice harness for the `src/speak` module (not part of the TUI).
//!
//! ```text
//! cargo speak pt-br -t "Olá, como vai você?"
//! cargo speak fr                    # sample sentence, first French voice
//! cargo speak ja -v jm_kumo -o /tmp/ja.wav
//! cargo speak --list                # every supported language
//! ```
//!
//! The binary needs the `speak-phonemize` feature (the alias in
//! `.cargo/config.toml` adds it), because only English and Chinese have a
//! lexicon in the Kokoro bundles; the other seven languages are phonemized by
//! espeak-ng before generation. The default bundle is
//! `kokoro-multi-lang-v1_0`, the only one with a voice for all nine languages.

use std::env;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use cosh::speak::{MODEL_DIR_ENV, Phonemizer, Speak, SpeakConfig, Voice, default_model_dir};

/// The bundle this harness uses by default. It is the only one shipping a
/// voice for all nine languages; the v1.1 bundle is the Chinese + English
/// `v1.1-zh` model (103 speakers).
const MULTILANG_BUNDLE: &str = "kokoro-multi-lang-v1_0";

const HELP: &str = "\
Kokoro voice test harness.

Usage:
  cargo speak <language> [-t TEXT] [options]
  cargo run --features speak-phonemize --bin speak -- <language> [-t TEXT] [options]

Options:
  -t, --text <TEXT>      text to speak (otherwise stdin, then a sample line)
  -v, --voice <NAME|ID>  catalog voice (e.g. pf_dora); defaults to the first
                         voice of the language
  -s, --speed <FACTOR>   pace, 1.0 natural and >1 faster (default 1.0)
  -o, --out <FILE>       write a WAV file instead of playing it
      --model-dir <DIR>  Kokoro bundle; defaults to $COSH_KOKORO_MODEL_DIR or
                         <cache>/cosh/models/kokoro-multi-lang-v1_0 (the only
                         bundle with a voice in every language)
  -l, --list             list the supported languages
      --voices           list this model's voices for the language
  -q, --quiet            print only errors
  -h, --help             show this help
";

/// One Kokoro language: which voice names it owns and how to phonemize it.
struct Language {
    /// Id used on the command line and in output, e.g. `pt-br`.
    id: &'static str,
    /// Human-readable name.
    label: &'static str,
    /// Extra accepted spellings (already normalized: lowercase, `_` as `-`).
    aliases: &'static [&'static str],
    /// Voice name prefixes (`pf_`, `pm_`, ...) in catalogue order.
    voices: &'static [&'static str],
    /// espeak-ng voice, or `None` for sherpa-onnx's own lexicon frontend.
    espeak: Option<&'static str>,
    /// Sentence spoken when no text is given.
    sample: &'static str,
}

const LANGUAGES: &[Language] = &[
    Language {
        id: "en-us",
        label: "English (United States)",
        aliases: &["en", "en-us", "english", "us"],
        voices: &["af", "am"],
        espeak: None,
        sample: "Hello, how are you? This is a voice test.",
    },
    Language {
        id: "en-gb",
        label: "English (United Kingdom)",
        aliases: &["en-gb", "english-gb", "gb", "british"],
        voices: &["bf", "bm"],
        espeak: None,
        sample: "Good afternoon. This is a voice test.",
    },
    Language {
        id: "zh",
        label: "Chinese (Mandarin)",
        aliases: &["zh", "zh-cn", "cn", "chinese", "mandarin"],
        voices: &["zf", "zm"],
        espeak: None,
        sample: "你好，这是一个语音测试。",
    },
    Language {
        id: "ja",
        label: "Japanese",
        aliases: &["ja", "jp", "japanese"],
        voices: &["jf", "jm"],
        espeak: Some("ja"),
        sample: "こんにちは、これはおんせいのてすとです。",
    },
    Language {
        id: "es",
        label: "Spanish",
        aliases: &["es", "spanish", "espanol"],
        voices: &["ef", "em"],
        espeak: Some("es"),
        sample: "Hola, ¿cómo estás? Esta es una prueba de voz.",
    },
    Language {
        id: "fr",
        label: "French",
        aliases: &["fr", "french", "francais"],
        voices: &["ff"],
        espeak: Some("fr"),
        sample: "Bonjour, comment allez-vous ? Ceci est un test de voix.",
    },
    Language {
        id: "hi",
        label: "Hindi",
        aliases: &["hi", "hindi"],
        voices: &["hf", "hm"],
        espeak: Some("hi"),
        sample: "नमस्ते, यह एक आवाज़ परीक्षण है।",
    },
    Language {
        id: "it",
        label: "Italian",
        aliases: &["it", "italian", "italiano"],
        voices: &["if", "im"],
        espeak: Some("it"),
        sample: "Ciao, come stai? Questo è un test della voce.",
    },
    Language {
        id: "pt-br",
        label: "Portuguese (Brazil)",
        aliases: &["pt", "pt-br", "portuguese", "portugues", "br"],
        voices: &["pf", "pm"],
        espeak: Some("pt-br"),
        sample: "Olá, como vai você? Este é um teste de voz.",
    },
];

struct Args {
    language: Option<&'static Language>,
    text: Option<String>,
    voice: Option<String>,
    speed: f32,
    out: Option<PathBuf>,
    model_dir: Option<PathBuf>,
    list: bool,
    voices: bool,
    quiet: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("speak: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() || raw.iter().any(|arg| arg == "-h" || arg == "--help") {
        print!("{HELP}");
        return Ok(());
    }

    let args = Args::parse(raw)?;
    if args.list {
        list_languages();
        return Ok(());
    }
    let language = args
        .language
        .ok_or_else(|| "missing <language>; run with --list to see them".to_string())?;

    let text = resolve_text(&args, language)?;
    if text.trim().is_empty() {
        return Err("the text to speak is empty".to_string());
    }

    let phonemizer = language
        .espeak
        .map_or(Phonemizer::Lexicon, |lang| Phonemizer::Piper {
            lang: lang.to_string(),
        });
    let config = SpeakConfig {
        model_dir: model_dir(&args),
        speed: args.speed,
        phonemizer,
        ..SpeakConfig::default()
    };

    if !args.quiet {
        println!("language: {} ({})", language.id, language.label);
        match language.espeak {
            None => println!("frontend: sherpa-onnx lexicon"),
            Some(code) => println!("frontend: espeak-ng `{code}` via piper-phonemize"),
        }
    }

    let mut speak = Speak::load(config).map_err(|error| error.to_string())?;

    if args.voices {
        return list_voices(&speak, language);
    }

    let sid = pick_voice(&speak, language, args.voice.as_deref())?;
    speak.set_voice(sid).map_err(|error| error.to_string())?;
    let name = voice_name(&speak, sid);

    if !args.quiet {
        println!(
            "voice: {name} (sid {sid}) · {} speakers · {} Hz",
            speak.num_speakers(),
            speak.sample_rate()
        );
        println!("text: {text}");
    }

    let started = Instant::now();
    let speech = speak.synthesize(&text).map_err(|error| error.to_string())?;
    let synthesis = started.elapsed();

    if !args.quiet {
        println!(
            "audio: {:.2}s ({} samples) synthesized in {:.2}s",
            speech.duration().as_secs_f64(),
            speech.samples().len(),
            synthesis.as_secs_f64()
        );
    }

    match args.out {
        Some(path) => {
            speech.save_wav(&path).map_err(|error| error.to_string())?;
            println!("saved {}", path.display());
        }
        None => speech.play().map_err(|error| {
            format!("{error}\n  hint: pass `-o audio.wav` to save the audio instead")
        })?,
    }
    Ok(())
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        let mut args = Self {
            language: None,
            text: None,
            voice: None,
            speed: 1.0,
            out: None,
            model_dir: None,
            list: false,
            voices: false,
            quiet: false,
        };
        let mut words = Vec::new();
        let mut iter = raw.into_iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--" => {
                    words.extend(iter.by_ref());
                    break;
                }
                "-t" | "--text" => args.text = Some(take_value(&mut iter, &arg)?),
                "-v" | "--voice" => args.voice = Some(take_value(&mut iter, &arg)?),
                "-o" | "--out" => args.out = Some(PathBuf::from(take_value(&mut iter, &arg)?)),
                "--model-dir" => {
                    args.model_dir = Some(PathBuf::from(take_value(&mut iter, &arg)?));
                }
                "-s" | "--speed" => {
                    let value = take_value(&mut iter, &arg)?;
                    args.speed = value
                        .parse::<f32>()
                        .map_err(|_| format!("`{value}` is not a valid speed factor"))?;
                }
                "-l" | "--list" => args.list = true,
                "--voices" => args.voices = true,
                "-q" | "--quiet" => args.quiet = true,
                flag if flag.starts_with('-') && flag.len() > 1 => {
                    return Err(format!("unknown option `{flag}`; run with --help"));
                }
                word => words.push(word.to_string()),
            }
        }

        if let Some(first) = words.first().cloned() {
            args.language = Some(
                find_language(&first)
                    .ok_or_else(|| format!("unsupported language `{first}`; run with --list"))?,
            );
            if args.text.is_none() && words.len() > 1 {
                args.text = Some(words[1..].join(" "));
            }
        }
        Ok(args)
    }
}

fn take_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("option `{flag}` needs a value"))
}

fn find_language(id: &str) -> Option<&'static Language> {
    let id = id.to_ascii_lowercase().replace('_', "-");
    LANGUAGES
        .iter()
        .find(|language| language.id == id || language.aliases.contains(&id.as_str()))
}

fn list_languages() {
    for language in LANGUAGES {
        let frontend = match language.espeak {
            None => "lexicon",
            Some(code) => code,
        };
        println!(
            "{:<6} {:<26} voices {:<8} frontend {}",
            language.id,
            language.label,
            language.voices.join("/"),
            frontend
        );
    }
}

fn list_voices(speak: &Speak, language: &Language) -> Result<(), String> {
    let mut found = false;
    for voice in matching_voices(speak, language) {
        println!("{:>4}  {}", voice.sid, voice.name);
        found = true;
    }
    if found {
        Ok(())
    } else {
        Err(format!(
            "this model ({} speakers) has no catalogue voices for {}",
            speak.num_speakers(),
            language.id
        ))
    }
}

fn matching_voices<'a>(
    speak: &'a Speak,
    language: &'a Language,
) -> impl Iterator<Item = Voice> + 'a {
    speak
        .voices()
        .iter()
        .copied()
        .filter(move |voice| language.voices.contains(&prefix(voice.name)))
}

fn prefix(name: &str) -> &str {
    name.split('_').next().unwrap_or(name)
}

/// Resolve the requested speaker: an explicit catalogue name or id wins,
/// otherwise the first voice the model has for the language.
fn pick_voice(speak: &Speak, language: &Language, requested: Option<&str>) -> Result<i32, String> {
    if let Some(requested) = requested {
        if let Ok(sid) = requested.parse::<i32>() {
            if sid < 0 || sid as u32 >= speak.num_speakers() {
                return Err(format!(
                    "voice id {sid} is out of range: this model has {} speakers",
                    speak.num_speakers()
                ));
            }
            return Ok(sid);
        }
        return speak
            .voice(requested)
            .map(|voice| voice.sid)
            .ok_or_else(|| {
                format!(
                    "unknown voice `{requested}` for this model; \
                     run `--voices` to list the ones it has"
                )
            });
    }

    matching_voices(speak, language)
        .next()
        .map(|voice| voice.sid)
        .ok_or_else(|| {
            format!(
                "this model ({} speakers) has no {} voices; use the \
                 `{MULTILANG_BUNDLE}` bundle or pick a speaker with -v <ID>",
                speak.num_speakers(),
                language.id
            )
        })
}

/// Where the harness looks for the model: `--model-dir`, then
/// `$COSH_KOKORO_MODEL_DIR`, then the multilingual v1.0 bundle next to the
/// library's default location.
fn model_dir(args: &Args) -> PathBuf {
    if let Some(dir) = &args.model_dir {
        return dir.clone();
    }
    if let Some(dir) = env::var_os(MODEL_DIR_ENV).filter(|value| !value.is_empty()) {
        return PathBuf::from(dir);
    }
    default_model_dir()
        .parent()
        .map(|models| models.join(MULTILANG_BUNDLE))
        .unwrap_or_else(default_model_dir)
}

fn voice_name(speak: &Speak, sid: i32) -> String {
    speak
        .voices()
        .iter()
        .find(|voice| voice.sid == sid)
        .map(|voice| voice.name.to_string())
        .unwrap_or_else(|| format!("speaker {sid}"))
}

fn resolve_text(args: &Args, language: &Language) -> Result<String, String> {
    if let Some(text) = &args.text {
        return Ok(text.clone());
    }
    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|error| format!("could not read stdin: {error}"))?;
        if !buffer.trim().is_empty() {
            return Ok(buffer.trim().to_string());
        }
    }
    Ok(language.sample.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        Args::parse(args.iter().map(|arg| (*arg).to_string()).collect())
    }

    #[test]
    fn the_language_table_is_consistent() {
        let mut ids: Vec<&str> = LANGUAGES.iter().map(|language| language.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), LANGUAGES.len(), "duplicate language id");

        for language in LANGUAGES {
            assert!(!language.voices.is_empty(), "{}", language.id);
            assert!(!language.sample.trim().is_empty(), "{}", language.id);
            // Only English and Chinese ship a lexicon in the Kokoro bundles.
            let lexicon = matches!(language.id, "en-us" | "en-gb" | "zh");
            assert_eq!(language.espeak.is_none(), lexicon, "{}", language.id);
        }
    }

    #[test]
    fn ids_aliases_and_underscores_resolve() {
        assert_eq!(find_language("pt-br").unwrap().id, "pt-br");
        assert_eq!(find_language("PT_BR").unwrap().id, "pt-br");
        assert_eq!(find_language("portugues").unwrap().id, "pt-br");
        assert_eq!(find_language("en").unwrap().id, "en-us");
        assert_eq!(find_language("jp").unwrap().id, "ja");
        assert!(find_language("xx").is_none());
    }

    #[test]
    fn text_comes_from_the_flag_then_from_the_trailing_words() {
        let args = parse(&["pt-br", "-t", "olá, como vai você?"]).unwrap();
        assert_eq!(args.language.unwrap().id, "pt-br");
        assert_eq!(args.text.as_deref(), Some("olá, como vai você?"));

        let args = parse(&["fr", "bonjour", "le", "monde"]).unwrap();
        assert_eq!(args.text.as_deref(), Some("bonjour le monde"));
    }

    #[test]
    fn options_are_parsed_and_bad_input_is_rejected() {
        let args = parse(&["ja", "-v", "jm_kumo", "-s", "1.2", "-o", "a.wav"]).unwrap();
        assert_eq!(args.voice.as_deref(), Some("jm_kumo"));
        assert_eq!(args.speed, 1.2);
        assert_eq!(args.out.unwrap(), PathBuf::from("a.wav"));

        assert!(parse(&["es", "--bogus"]).is_err());
        assert!(parse(&["xx"]).is_err());
        assert!(parse(&["es", "-t"]).is_err());
    }
}
