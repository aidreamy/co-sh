//! The `morpheus-decisiond` binary.
//!
//! A thin shim over [`morpheus::daemon::decision::run`]: all the logic (bind,
//! serve, idle exit, the bind-race outcome) lives in the library so the
//! integration tests exercise the exact code the daemon runs.

fn main() {
    let code = morpheus::daemon::decision::run();
    std::process::exit(code);
}
