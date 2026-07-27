//! PROTOTYPE — throwaway. Answers #8 only; delete once the ADR lands.
//!
//! One crate, three platforms: `dx serve --platform {desktop,web,android}`.

mod model;
mod store;

#[cfg(target_os = "android")]
mod android;

use dioxus::prelude::*;
use model::{ReviewEvent, CARDS, GRADES};

#[cfg(target_os = "android")]
const DEVICE: &str = "dioxus-android";
#[cfg(all(target_family = "wasm", not(target_os = "android")))]
const DEVICE: &str = "dioxus-web";
#[cfg(not(any(target_os = "android", target_family = "wasm")))]
const DEVICE: &str = "dioxus-desktop";

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut idx = use_signal(|| 0usize);
    let mut revealed = use_signal(|| false);
    let mut log = use_signal(Vec::<ReviewEvent>::new);
    let mut status = use_signal(|| "opening store…".to_string());

    // Restart survival: everything on screen below the fold comes from the log on disk.
    use_future(move || async move {
        match store::Store::open().await {
            Ok(s) => match s.read_all().await {
                Ok(evs) => {
                    log.set(evs);
                    status.set(store::Store::backend());
                }
                Err(e) => status.set(format!("read failed: {e}")),
            },
            Err(e) => status.set(format!("open failed: {e}")),
        }
    });

    let grade = move |g: u8| {
        spawn(async move {
            let ev = ReviewEvent {
                card_id: idx() as u32,
                grade: g,
                at_ms: store::now_ms(),
                device: DEVICE.to_string(),
            };
            match store::Store::open().await {
                Ok(s) => match s.append(&ev).await {
                    Ok(()) => match s.read_all().await {
                        Ok(evs) => log.set(evs),
                        Err(e) => status.set(format!("read failed: {e}")),
                    },
                    Err(e) => status.set(format!("append failed: {e}")),
                },
                Err(e) => status.set(format!("open failed: {e}")),
            }
            revealed.set(true);
        });
    };

    let card = &CARDS[idx() % CARDS.len()];

    rsx! {
        style { {CSS} }
        main {
            h1 { "Dioxus slice · {DEVICE}" }

            div { class: "card",
                div { class: "front", "{card.front}" }
                if revealed() {
                    div { class: "back", "{card.back}" }
                    button { class: "next",
                        onclick: move |_| { idx += 1; revealed.set(false); },
                        "Next card"
                    }
                } else {
                    div { class: "grades",
                        for (g, label) in GRADES {
                            button {
                                key: "{g}",
                                class: if g == 1 { "grade fail" } else { "grade" },
                                onclick: move |_| grade(g),
                                span { class: "n", "{g}" }
                                "{label}"
                            }
                        }
                    }
                }
            }

            section { class: "log",
                h2 { "Persisted log — {log().len()} events" }
                p { class: "backend", "{status()}" }
                ul {
                    for ev in log().iter().rev().take(6) {
                        li { key: "{ev.at_ms}",
                            "card {ev.card_id} · grade {ev.grade} · {ev.at_ms} · {ev.device}"
                        }
                    }
                }
                p { class: "hint", "Restart the app. The log must survive." }
            }
        }
    }
}

const CSS: &str = r#"
* { box-sizing: border-box; }
body { margin: 0; font: 16px/1.5 system-ui, sans-serif; background: #14161a; color: #e6e8ec; }
main { max-width: 34rem; margin: 0 auto; padding: 1.5rem 1rem 3rem; }
h1 { font-size: 0.85rem; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase;
     color: #7f8894; margin: 0 0 1rem; }
.card { background: #1c1f26; border: 1px solid #2a2f39; border-radius: 14px; padding: 1.75rem 1.25rem; }
.front { font-size: 1.9rem; font-weight: 600; text-align: center; padding: 1.5rem 0; }
.back { font-size: 1.2rem; text-align: center; color: #7ee2b9; border-top: 1px solid #2a2f39;
        padding: 1.25rem 0 0.5rem; }
.grades { display: grid; grid-template-columns: 1fr; gap: 0.5rem; margin-top: 1rem; }
.grade { display: flex; align-items: center; gap: 0.75rem; width: 100%; padding: 0.9rem 1rem;
         font: inherit; color: inherit; background: #252932; border: 1px solid #333945;
         border-radius: 10px; cursor: pointer; }
.grade:active { background: #2f3541; }
.grade.fail { margin-bottom: 0.6rem; border-color: #6b3239; background: #2a1e21; }
.n { display: inline-grid; place-items: center; width: 1.6rem; height: 1.6rem; flex: none;
     border-radius: 6px; background: #3a4150; font-size: 0.8rem; font-weight: 700; }
.next { width: 100%; margin-top: 1rem; padding: 0.9rem; font: inherit; font-weight: 600;
        color: #14161a; background: #7ee2b9; border: 0; border-radius: 10px; cursor: pointer; }
.log { margin-top: 2rem; }
.log h2 { font-size: 0.95rem; margin: 0 0 0.25rem; }
.backend { margin: 0 0 0.75rem; font-size: 0.72rem; color: #7f8894; word-break: break-all; }
.log ul { list-style: none; margin: 0; padding: 0; }
.log li { font-family: ui-monospace, monospace; font-size: 0.72rem; color: #9aa3b0;
          padding: 0.3rem 0; border-bottom: 1px solid #22262e; }
.hint { font-size: 0.75rem; color: #7f8894; margin-top: 0.9rem; }
"#;
