//! The slice's whole domain. Shared verbatim with the Tauri+Leptos slice so the two are comparable.

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

/// No scheduling. Three hardcoded cards is what the ticket asked for.
pub const CARDS: [Card; 3] = [
    Card { front: "das Gleichgewicht", back: "the balance / equilibrium" },
    Card { front: "verschwinden", back: "to disappear" },
    Card { front: "die Wahrscheinlichkeit", back: "the probability" },
];

/// ADR-0001: four grades, worded as recall outcomes, with a visual break between 1 and 2.
pub const GRADES: [(u8, &str); 4] =
    [(1, "Forgot"), (2, "Barely"), (3, "Good"), (4, "Easy")];
