//! Blocking playback through the audio player the machine already has.
//!
//! sherpa-onnx generates samples but intentionally opens no audio device; the
//! crate has no player. Linking `cpal`/`rodio` would put an ALSA development
//! header (`libasound2-dev`) in the build requirements of every platform, so
//! `speak` writes the utterance to a temporary WAV and hands it to the
//! platform's own command-line player instead. The dependency-free result
//! covers the desktop cases the TUI needs; a caller that wants gapless,
//! chunk-by-chunk output can drive [`Speak::synthesize_streaming`] with its
//! own sink.
//!
//! [`Speak::synthesize_streaming`]: super::Speak::synthesize_streaming

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use super::config::SpeakError;
use super::engine::Speech;

/// Serializes the temporary file names of the utterances played at once.
static COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn play(speech: &Speech) -> Result<(), SpeakError> {
    let path = temp_path();
    if let Err(error) = speech.save_wav(&path) {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    let result = play_file(&path);
    let _ = fs::remove_file(&path);
    result
}

fn temp_path() -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "morpheus-speak-{}-{unique}.wav",
        std::process::id()
    ))
}

/// Try each player in turn. A player that is absent is skipped; one that
/// exits unsuccessfully (no sound server, closed device) is recorded and the
/// next candidate gets a chance.
fn play_file(path: &Path) -> Result<(), SpeakError> {
    let mut tried = Vec::new();
    for mut command in commands(path) {
        let name = format!("{}", command.get_program().to_string_lossy());
        match command.status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => tried.push(format!("{name}: {status}")),
            Err(error) => tried.push(format!("{name}: {error}")),
        }
    }
    Err(SpeakError::NoPlayer(tried.join(", ")))
}

#[cfg(target_os = "macos")]
fn commands(path: &Path) -> Vec<Command> {
    let mut afplay = Command::new("afplay");
    afplay.arg(path);
    let mut ffplay = ffplay(path);
    vec![afplay, ffplay]
}

#[cfg(windows)]
fn commands(path: &Path) -> Vec<Command> {
    // `SoundPlayer.PlaySync` blocks like the Unix players do. Single quotes
    // are doubled to stay literal inside the PowerShell string.
    let literal = path.to_string_lossy().replace('\'', "''");
    let mut powershell = Command::new("powershell");
    powershell.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &format!("(New-Object Media.SoundPlayer '{literal}').PlaySync()"),
    ]);
    vec![powershell]
}

#[cfg(not(any(target_os = "macos", windows)))]
fn commands(path: &Path) -> Vec<Command> {
    let mut paplay = Command::new("paplay");
    paplay.arg(path);

    let mut pw_play = Command::new("pw-play");
    pw_play.arg(path);

    let mut aplay = Command::new("aplay");
    aplay.args(["-q"]).arg(path);

    let mut play = Command::new("play");
    play.args(["-q"]).arg(path);

    vec![paplay, pw_play, aplay, play, ffplay(path)]
}

#[cfg(not(windows))]
fn ffplay(path: &Path) -> Command {
    let mut command = Command::new("ffplay");
    command
        .args(["-nodisp", "-autoexit", "-loglevel", "quiet"])
        .arg(path);
    command
}
