//! PROTOTYPE domain — throwaway. See ../PROTOTYPE.md.
//!
//! Deliberately not real scheduling: box numbers and "due" flags are hand-authored per
//! [`Scenario`] so every variant can be judged against normal / empty / new-deck / backlog data
//! without implementing FSRS here. Real scheduling is ADR-0001; the event log format is
//! ADR-0004. This crate only needs *a* due queue and *a* box number to react to the UI.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct Card {
    pub id: u32,
    pub front: String,
    pub back: String,
    /// Display-only fact (constraint 4: `box = f(stability)`, durability not urgency).
    /// `0` means "new — never reviewed", shown as "new" rather than "box 0".
    pub box_num: u8,
    pub due: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scenario {
    Normal,
    Empty,
    NewDeck,
    Backlog,
}

impl Scenario {
    pub const ALL: [Scenario; 4] = [Scenario::Normal, Scenario::Empty, Scenario::NewDeck, Scenario::Backlog];

    pub fn label(self) -> &'static str {
        match self {
            Scenario::Normal => "Normal",
            Scenario::Empty => "Empty",
            Scenario::NewDeck => "New deck",
            Scenario::Backlog => "Backlog",
        }
    }
}

/// The fixed 8-card deck shared by Normal / Empty / New-deck. One Persian card keeps the bidi
/// obligation live in this prototype too (AGENTS.md rule 1).
fn base_deck() -> Vec<(u32, &'static str, &'static str)> {
    vec![
        (1, "das Gleichgewicht", "the balance / equilibrium"),
        (2, "verschwinden", "to disappear"),
        (3, "die Wahrscheinlichkeit", "the probability"),
        (4, "خوشبختانه", "fortunately"),
        (5, "die Erfahrung", "the experience"),
        (6, "widerspruchlich", "contradictory"),
        (7, "die Verantwortung", "the responsibility"),
        (8, "erwähnen", "to mention"),
    ]
}

/// All cards for a scenario, due-flagged and box-numbered. Not filtered against the log —
/// [`due_queue`] does that.
pub fn all_cards(scenario: Scenario) -> Vec<Card> {
    match scenario {
        Scenario::Normal => {
            // Mixed boxes, five of eight due today.
            let boxes = [2u8, 4, 1, 3, 5, 2, 1, 4];
            let due = [true, true, false, true, true, false, true, false];
            base_deck()
                .into_iter()
                .enumerate()
                .map(|(i, (id, f, b))| Card {
                    id,
                    front: f.to_string(),
                    back: b.to_string(),
                    box_num: boxes[i],
                    due: due[i],
                })
                .collect()
        }
        Scenario::Empty => {
            // Same deck, everything freshly reviewed — nothing due.
            let boxes = [3u8, 3, 4, 2, 5, 3, 4, 2];
            base_deck()
                .into_iter()
                .enumerate()
                .map(|(i, (id, f, b))| Card { id, front: f.to_string(), back: b.to_string(), box_num: boxes[i], due: false })
                .collect()
        }
        Scenario::NewDeck => base_deck()
            .into_iter()
            .map(|(id, f, b)| Card { id, front: f.to_string(), back: b.to_string(), box_num: 0, due: true })
            .collect(),
        Scenario::Backlog => {
            // Synthetic cards, ids offset so they never collide with the fixed deck. Boxes
            // skewed low — a backlog is disproportionately cards that keep getting missed.
            (0..150u32)
                .map(|i| {
                    let box_num = match i % 5 {
                        0 | 1 => 1,
                        2 | 3 => 2,
                        _ => 3,
                    };
                    Card {
                        id: 1000 + i,
                        front: format!("backlog card {}", i + 1),
                        back: format!("answer {}", i + 1),
                        box_num,
                        due: true,
                    }
                })
                .collect()
        }
    }
}

/// The due queue: this scenario's due cards minus anything already graded in the log — no
/// separate "session progress" is stored anywhere, so this is the *only* place session position
/// lives. Reload the log from disk and this reproduces itself exactly, which is the point:
/// killing the app mid-session loses nothing because nothing besides the log held the position.
pub fn due_queue(scenario: Scenario, log: &[ReviewEvent]) -> Vec<Card> {
    let graded: std::collections::HashSet<u32> = log.iter().map(|e| e.card_id).collect();
    all_cards(scenario).into_iter().filter(|c| c.due && !graded.contains(&c.id)).collect()
}

/// Illustrative only — not FSRS. Just enough spread that the four grade buttons in variant B
/// show visibly different numbers, which is what the "is this noise" question needs to react to.
pub fn projected_interval_days(box_num: u8, grade: u8) -> u32 {
    let base = (box_num.max(1) as u32) * 2;
    match grade {
        1 => 0,
        2 => base / 2,
        3 => base,
        4 => base * 2 + 3,
        _ => base,
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReviewEvent {
    pub card_id: u32,
    pub grade: u8,
    pub at_ms: i64,
}

/// A one-line callout for scenarios where the bare due-count would be misleading on its own.
pub fn scenario_note(scenario: Scenario) -> Option<&'static str> {
    match scenario {
        Scenario::NewDeck => Some("Fresh deck — 0 reviews logged yet, so everything below is a first look."),
        Scenario::Backlog => Some("This deck has fallen behind — 150 cards are overdue."),
        _ => None,
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
