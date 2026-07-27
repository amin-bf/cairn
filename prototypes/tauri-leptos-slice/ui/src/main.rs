//! PROTOTYPE — throwaway. Answers #8 only; delete once the ADR lands.
//!
//! Leptos 0.8 CSR. Served by Tauri on desktop/Android, and as a plain static SPA on web.

mod store;

use leptos::prelude::*;
use leptos::task::spawn_local;
use slice_shared::{ReviewEvent, CARDS, GRADES};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (idx, set_idx) = signal(0usize);
    let (revealed, set_revealed) = signal(false);
    let (log, set_log) = signal(Vec::<ReviewEvent>::new());
    let (status, set_status) = signal(String::from("opening store…"));

    // Restart survival: everything below the fold comes from the log on disk.
    spawn_local(async move {
        set_status.set(store::backend().await);
        match store::read_all().await {
            Ok(evs) => set_log.set(evs),
            Err(e) => set_status.set(format!("read failed: {e}")),
        }
    });

    let grade = move |g: u8| {
        spawn_local(async move {
            let ev = ReviewEvent {
                card_id: idx.get_untracked() as u32,
                grade: g,
                at_ms: store::now_ms(),
                device: store::device(),
            };
            if let Err(e) = store::append(&ev).await {
                set_status.set(format!("append failed: {e}"));
            }
            match store::read_all().await {
                Ok(evs) => set_log.set(evs),
                Err(e) => set_status.set(format!("read failed: {e}")),
            }
            set_revealed.set(true);
        });
    };

    view! {
        <style>{CSS}</style>
        <main>
            <h1>"Leptos+Tauri slice · " {store::device()}</h1>

            <div class="card">
                <div class="front">{move || CARDS[idx.get() % CARDS.len()].front}</div>
                <Show
                    when=move || revealed.get()
                    fallback=move || view! {
                        <div class="grades">
                            {GRADES.iter().map(|&(g, label)| view! {
                                <button
                                    class=if g == 1 { "grade fail" } else { "grade" }
                                    on:click=move |_| grade(g)
                                >
                                    <span class="n">{g}</span>
                                    {label}
                                </button>
                            }).collect_view()}
                        </div>
                    }
                >
                    <div class="back">{move || CARDS[idx.get() % CARDS.len()].back}</div>
                    <button
                        class="next"
                        on:click=move |_| { set_idx.update(|i| *i += 1); set_revealed.set(false); }
                    >
                        "Next card"
                    </button>
                </Show>
            </div>

            <section class="log">
                <h2>"Persisted log — " {move || log.get().len()} " events"</h2>
                <p class="backend">{move || status.get()}</p>
                <ul>
                    {move || log.get().iter().rev().take(6).map(|ev| view! {
                        <li>{format!(
                            "card {} · grade {} · {} · {}",
                            ev.card_id, ev.grade, ev.at_ms, ev.device
                        )}</li>
                    }).collect_view()}
                </ul>
                <p class="hint">"Restart the app. This list must survive."</p>
            </section>
        </main>
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
