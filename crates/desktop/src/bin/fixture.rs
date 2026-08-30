//! `cairn-fixture` — install one of the bench's pre-made collections into the platform's data
//! directory, then get out of the way.
//!
//! ```text
//! cairn-fixture caught-up
//! ```
//!
//! It exists because the capture harness redirects `XDG_DATA_HOME` and `XDG_STATE_HOME` into a
//! scratch profile it wipes per run, which is *why* every capture is a first launch and always the
//! same six seeded cards. Run before the app starts, this leaves a collection already in the wanted
//! state, so the app opens a non-empty store and the shipping seed never fires — see
//! `cairn_app::fixtures` for why that route was chosen over the two obvious alternatives.
//!
//! **Keep it this short**, for the same reason `main.rs` is short (ADR-0003 §5): logic here is never
//! compiled by the Android build and never exercised on the handset. Everything real is in
//! `cairn_app::fixtures`, which the temporary Settings block calls too — one definition of each
//! state, two ways in.
//!
//! **It fails loudly or not at all.** An unknown name, a store that will not open, or a fixture that
//! did not land where it says it lands all exit non-zero, so `capture-desktop.sh` aborts rather than
//! photographing the seed under the fixture's name. A storyboard that misses its target fails
//! silently and has done so twice (#122, #143); this is the one place that class of failure can be
//! caught by a machine.

use cairn_app::fixtures::Fixture;

fn main() -> std::process::ExitCode {
    let Some(name) = std::env::args().nth(1) else {
        eprintln!("usage: cairn-fixture <name>\n{}", catalogue());
        return std::process::ExitCode::FAILURE;
    };

    let Some(fixture) = Fixture::parse(&name) else {
        eprintln!("cairn-fixture: no fixture called '{name}'\n{}", catalogue());
        return std::process::ExitCode::FAILURE;
    };

    match cairn_app::fixtures::install_into_platform_dirs(fixture) {
        Ok(reached) => {
            println!("cairn-fixture: {} — {reached}", fixture.key());
            std::process::ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("cairn-fixture: {message}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn catalogue() -> String {
    Fixture::ALL
        .into_iter()
        .map(|f| format!("  {:<12} {}", f.key(), f.reaches()))
        .collect::<Vec<_>>()
        .join("\n")
}
