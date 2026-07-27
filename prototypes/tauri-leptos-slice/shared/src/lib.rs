//! The portable third crate the research describes: pure domain, depended on by BOTH the wasm
//! frontend and the native Tauri core. Nothing here may touch the filesystem — that is the whole
//! point of the split.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReviewEvent {
    pub card_id: u32,
    pub grade: u8,
    pub at_ms: i64,
    pub device: String,
}

pub struct Card {
    pub front: &'static str,
    pub back: &'static str,
}

pub const CARDS: [Card; 3] = [
    Card { front: "das Gleichgewicht", back: "the balance / equilibrium" },
    Card { front: "verschwinden", back: "to disappear" },
    Card { front: "die Wahrscheinlichkeit", back: "the probability" },
];

pub const GRADES: [(u8, &str); 4] =
    [(1, "Forgot"), (2, "Barely"), (3, "Good"), (4, "Easy")];

pub fn parse_log(text: &str) -> Vec<ReviewEvent> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}
