# Leitner boxes as a UI over an interval scheduler — verifiable facts

Research date: 2026-07-26. Every claim below carries a URL. Claims are tagged:

- **[PRIMARY]** — traceable to the originator (Leitner's book, a peer-reviewed paper, official app docs, or the actual source code).
- **[SECONDARY]** — a reputable third party asserting something about a primary source.
- **[FOLKLORE]** — widely repeated, no traceable origin found.

A hard caveat up front: **I could not read Leitner's book *So lernt man lernen* itself.** Every statement about "the original Leitner system" below is therefore at best [SECONDARY]. This matters more than it sounds — see §1.4.

---

## 1. The Leitner system as published

### 1.1 Bibliographic facts (mostly settled, with one inconsistency)

- Sebastian Leitner, German science journalist. The book was first published **1972** under the title *Lernen lernen*; the current title is *So lernt man lernen*. German Wikipedia: "Dieses System entwickelte Sebastian Leitner, der es 1972 in seinem Schlüsselwerk *Lernen lernen* vorstellte (heutiger Titel: *So lernt man lernen*)" — https://de.wikipedia.org/wiki/Lernkartei **[SECONDARY]**
- English Wikipedia agrees on 1972 and cites the 18th edition: "Leitner, Sebastian. *So lernt man lernen*. Freiburg: Verlag Herder, 2011 (18th ed.), ISBN 978-3-451-05060-2" — https://en.wikipedia.org/wiki/Leitner_system **[SECONDARY]**
- **Inconsistency in the academic literature.** Settles & Meeder (ACL 2016) cite "S. Leitner. 1972. *So lernt man lernen*. Angewandte…" — https://research.duolingo.com/papers/settles.acl16.pdf. Reddy et al. (KDD 2016) cite "[13] S. Leitner. *So lernt man lernen*. Herder, 1974." **and in prose say the system was "first introduced in 1970"** — https://www.cs.cornell.edu/~tj/publications/reddy_etal_16c.pdf. So three different years (1970 / 1972 / 1974) appear in citable literature. Nobody cites a page number. **[unresolved]**

### 1.2 What the original actually specified: box *capacity*, not per-box intervals

This is the single most important finding for the ticket.

English Wikipedia, *Method* section: *"In Leitner's original method, published in his book So lernt man Lernen (How to learn to learn), the schedule of repetition was governed by the size of the partitions in the learning box. These were 1, 2, 5, 8, and 14 cm. Only when a partition became full was the learner to review some of the cards it contained, moving them forward or back depending on whether they remembered them."* — https://en.wikipedia.org/wiki/Leitner_system **[SECONDARY]**

Reading that literally:

- **5 partitions**, sized 1 / 2 / 5 / 8 / 14 cm.
- The trigger for review is **"partition became full"** — i.e. a *capacity/backpressure* rule, not a per-card due date and not a per-box interval.
- The intervals are therefore **emergent**, not specified: how long a card waits depends on how fast that partition fills, which depends on deck size, study rate, and the learner's accuracy.
- The promotion/demotion rule is stated only as "moving them forward or back depending on whether they remembered them" — a **binary** signal, and the direction on failure ("back") is not pinned down to "back one" vs "back to box 1" in this sentence.

**Caveat on the sourcing of this passage:** I checked the article's wikitext (https://en.wikipedia.org/w/index.php?title=Leitner_system&action=raw). The 1/2/5/8/14 cm sentence has **no inline `<ref>` of its own**. The article carries a `{{More citations needed}}` maintenance tag dated April 2023, and its lead-sentence footnotes are Medium posts and study blogs, not Leitner. The Leitner book appears once as a bare citation supporting the lead. So even this most-specific claim about the original is not properly footnoted.

### 1.3 The German-language description: proportional sampling, again not intervals

German Wikipedia's worked example is a **rate-based / sampling** scheduler, which is a different mechanism again from both "per-box interval" and "partition full":

> "Ein Beispiel: drei Fächer: 1, 2 und 3. […] Die Karten im Fach 1 werden täglich wiederholt, die Karten im Fach 2 jeden zweiten Tag und die Karten im Fach 3 jeden vierten. **Für jeden Lernvorgang werden somit alle Karten aus Fach 1, die Hälfte der Karten aus Fach 2 und ein Viertel der Karten aus Fach 3 zufällig ausgewählt.**"
> ("For each study session, all cards from box 1, half the cards from box 2, and a quarter of the cards from box 3 are selected at random.")
> — https://de.wikipedia.org/wiki/Lernkartei **[SECONDARY]**

Other details from the same source:

- Demotion is **to box 1**, not one step back: "War die Lösung bei erster Vorlage nicht bekannt, so wandert sie nicht in das jeweils nächste, sondern wieder ganz nach vorne in Fach 1."
- Promotion requires being correct **on first presentation** in the session; within-session repeats until mastered do not earn promotion: "Damit eine Karteikarte in das nächste Fach wandert, muss der Kandidat somit die Antwort bei erstmaliger Vorlage wissen."
- Only a **minimum session gap** is specified, no per-box interval: "sollte zwischen den Abfragen genügend Zeit (mindestens 8 Stunden) liegen."
- Boxes should have **increasing physical capacity**, box 1 being small (20–30 cards): "Das erste Fach ist sehr klein (20 bis 30 Karteikarten), das zweite Fach schon etwas größer…" — consistent with §1.2's capacity-driven reading.
- Like the English article, none of this is inline-cited to Leitner. The German article's *Literatur* section lists Leitner's 18th ed. plus four German study-technique books (Beelich & Schwede 1979; Born & Oehler 2011; Fenske 2002 — *Bio-logisch lernen mit der 5-Fächer-Lernbox*; Krueger 2014, pp. 153–163). Krueger is the only one cited inline anywhere.

### 1.4 Bottom line on Q1

**Answer to "does the original specify review scheduling, or only the box-shuffling rule?": the best available evidence says it specifies neither a per-box interval nor a per-card due date.** It specifies (a) a box-shuffling rule on a binary signal, and (b) a *capacity* rule that indirectly determines when a box gets reviewed. Timing is a consequence of physical geometry, not a schedule.

**What is folklore:** the near-universal "Box 1 = 1 day, Box 2 = 2 days, Box 3 = 4 days…" table presented as "the Leitner system". I found **no source that traces any specific day-interval table to Leitner's book.** See §5 for where those numbers actually come from.

Piotr Woźniak (SuperMemo) makes the same point from the other direction:

> "Leitner system is often incorrectly labelled as a spaced repetition system. Spaced repetition is computational in nature. Without an effort to compute optimum intervals, prioritized review is little more than an inefficient form of scheduling… In other words, Leitner is a hit-or-miss system."
> — https://supermemo.guru/wiki/Leitner_system **[PRIMARY as Woźniak's opinion]** (403s to plain fetchers; retrieved with a browser user-agent; page last edited 2018-11-21)

---

## 2. Is there an established mapping of a 4-point grade scale (Again/Hard/Good/Easy) onto box movement?

**Plain finding: no. I found no published, peer-reviewed, or standards-like specification mapping a 4-point grade scale onto Leitner box movement. Every graded-Leitner scheme I found in the wild is ad-hoc, and the schemes contradict each other.**

The evidence is not merely absence — it is that the serious literature *deliberately collapses grades to binary* when it touches Leitner.

### 2.1 The main academic Leitner paper explicitly discretises grades to binary

Reddy, Labutov, Banerjee & Joachims, **"Unbounded Human Learning: Optimal Scheduling for Spaced Repetition", KDD 2016** — https://arxiv.org/abs/1602.07032, PDF: https://www.cs.cornell.edu/~tj/publications/reddy_etal_16c.pdf. This is the "Leitner queue network" paper; code at https://github.com/rddy/leitnerq. **[PRIMARY]**

Their data source *had* a 6-point scale and they threw it away:

> "Each interaction is annotated with a grade (on a 0-5 scale) that was self-reported by the user. Users are instructed by the Mnemosyne software to use a grade of 0 or 1 to indicate that they did not recall the item, and a grade of 2-5 to indicate that they did recall the item, with higher grades implying easier recall. **We discretize grades into binary outcomes, where recall ≜ grade ≥ 2**, and observe an overall recall rate of 0.56 in the data."

Their box-movement rule is strictly binary and **one step in each direction**:

> "It comprises of a series of n decks of flashcards, indexed as {1, 2, …, n}, where new items enter the system at deck 1, and items upon being reviewed either move up a deck if recalled correctly or down if forgotten. […] items in deck 1 are reflected (i.e., they remain in deck 1 if they are incorrectly reviewed), and all items which are recalled at deck n (which in experiments we take as n = 5), are declared to be 'mastered' and removed from the system."

Transition probabilities (their Eqn. 3-region):

```
P[k → k+1]              = exp(−θ · D_k / k)
P[k → max{k−1, 1}]      = 1 − exp(−θ · D_k / k)
```

and they are blunt that per-box timing has never had a principled basis:

> "Existing schemes for assigning review frequencies to different decks are based on heuristics that are not founded on any formal reasoning, and hence, have no optimality guarantees."
> "…all existing schemes for assigning review frequencies to decks in the Leitner system, and in fact, in all other spaced repetition systems, are based on heuristics with no formal optimality guarantees."

Note also that their model is a **rate/queue** model, not a due-date model: each deck *k* gets a service rate μ_k, new items arrive at rate λ_ext, subject to λ_ext + Σμ_k ≤ U (the learner's review-frequency budget). It is a Jackson network of M/M/1 queues. This is much closer to §1.3's proportional-sampling picture than to "box 3 = 7 days".

### 2.2 The other academic treatment also treats Leitner as binary

Settles & Meeder, **"A Trainable Spaced Repetition Model for Language Learning", ACL 2016** (Duolingo) — https://research.duolingo.com/papers/settles.acl16.pdf **[PRIMARY]**. They formalise Leitner as a special case of their half-life regression, with a **two-feature binary** parameterisation:

> "Analyzing the Leitner variant from Figure 3 is even simpler: this corresponds to Θ = {x⊕ : 1, x⊖ : -1}, where x⊕ is the number of past correct responses (i.e., doubling the interval), and x⊖ is the number of incorrect responses (i.e., halving the interval)."

Combined with their ĥ_Θ = 2^(Θ·x), this yields **half-life = 2^(#correct − #incorrect)**. That is a genuinely published box↔interval identity — but the "box" is a *net binary streak*, with no place for Hard or Easy. (See §4.3, this is one of only two principled box↔interval derivations I found.)

### 2.3 Open-source: the reference implementation is binary by design

`open-spaced-repetition/leitner-box` (v0.3.0), from the same org that maintains FSRS — https://github.com/open-spaced-repetition/leitner-box. Source read directly: `src/leitner_box/leitner_box.py`. **[PRIMARY]**

```python
class Rating(IntEnum):
    Fail = 0
    Pass = 1
```

```python
if rating == Rating.Fail:
    if self.on_fail == "first_box":
        new_card.box = 1
    elif self.on_fail == "prev_box" and new_card.box > 1:
        new_card.box -= 1
elif rating == Rating.Pass:
    if new_card.box < len(self.box_intervals):
        new_card.box += 1
```

- Two ratings only. **No 4-point scale.**
- `on_fail` is a *configuration choice* between `"first_box"` (default) and `"prev_box"` — i.e. the org shipping the canonical implementation treats the demotion rule as genuinely undetermined by the source material.
- Defaults `box_intervals = [1, 2, 7]` (3 boxes), with a hard constraint `if box_intervals[0] != 1: raise ValueError("Box 1 must have an interval of 1 day. This may change in future versions.")`.

`nickhnsn/facharbeit-spaced-repetition` — a Java library that implements **Leitner, SM-2, and FSRS side by side** — https://github.com/nickhnsn/facharbeit-spaced-repetition. Its `FSRSRating` is a 4-point enum, but `LeitnerAlgorithm.java` takes `boolean retrievalSuccessful` and nothing else: **[PRIMARY]**

```java
if (this.retrievalSuccessful) { boxId = this.boxId + 1; }   // else boxId stays 1 (reset)
switch (boxId) { case 1: interval = 1; case 2: 3; case 3: 7; case 4: 30; case 5: 30*6; }
```

This is telling: an author who had a 4-point scale available for FSRS **chose to keep Leitner binary**.

### 2.4 Ad-hoc graded schemes that do exist in shipped/public code — and they disagree

Found via GitHub code search. These are the closest thing to a "graded Leitner spec" and they are mutually incompatible:

**(a) `yro7/panglot-public`, `core/src/srs/leitner.rs`** — https://github.com/yro7/panglot-public/blob/main/core/src/srs/leitner.rs **[PRIMARY]**

```rust
const BOX_INTERVALS: [f64; 5] = [1.0, 3.0, 7.0, 14.0, 30.0];
const fn next_box(current: usize, rating: Rating) -> usize {
    match rating {
        Rating::Again => 0,
        Rating::Hard  => if current == 0 { 0 } else { current - 1 },
        Rating::Good | Rating::Easy => if current >= MAX_BOX { MAX_BOX } else { current + 1 },
    }
}
```
Again → box 0 (reset). Hard → **demote one**. Good and Easy → **identical**, promote one. So Easy carries zero information, and Hard is treated as a failure.

**(b) `SouichiroTsujimoto/xanki`, `docs/spec/leitner-study.md`** — https://github.com/SouichiroTsujimoto/xanki/blob/main/docs/spec/leitner-study.md **[PRIMARY, written spec]** (Japanese). Review-phase behaviour:

| grade | box effect | interval used |
|---|---|---|
| `0` もう一度 / Again | → relearning phase | relearning step[0] |
| `1` 難しい / Hard | **box unchanged** (箱据置) | `hardInterval` (default 1 day) |
| `2` 正解 / Good | **box +1** (max 5) | that box's review interval |
| `3` 簡単 / Easy | **box +2** (max 5) | that box's review interval |

Review boxes are 2..5 with intervals **1, 3, 7, 21 days**; graduating interval 1 day, easy interval 4 days.

So across two real graded implementations: Hard is *demote one* in (a) and *no change* in (b); Easy is *identical to Good* in (a) and *skip a box* in (b). There is no convention.

**(c) Note what xanki's own spec says about showing the box to users:**

> "開発者向け用語 **Box**（`review_state.box`）。**UI では非表示。**"
> ("**Box** is a developer-facing term (`review_state.box`). **Not displayed in the UI.**")

An app that literally names its feature "Leitner" keeps the box number internal and shows Anki-style next-interval labels on the buttons instead. That is a data point directly on the ticket's question.

### 2.5 Woźniak's "Normalized Leitner" — a spec, but still binary

The one attempt I found at a normative Leitner spec: **[PRIMARY as Woźniak's own definition]** — https://supermemo.guru/wiki/Leitner_system

> "I use the term **Normalized Leitner** to refer to a software implementation with adjustments that can turn Leitner into a spaced repetition system […] This is how Normalized Leitner works:
> - boxes are associated with intervals
> - first interval is set to Int1, and successive intervals are set to Int1*power(E-Factor, repetition)
> - **failure results in reversal to box #1 (violating this principle worsens the performance of the algorithm)**
> - target recall at review claim is set to 90%"

Binary again, and it explicitly calls one-step-back demotion wrong — the page captions the one-step-back diagram as *"An **incorrect mutation** of the Leitner system where failed answers are moved back by one box only… This variant was in use in Duolingo for a while."*

### 2.6 Verdict on Q2

**Graded Leitner is always ad-hoc.** There is no published mapping. The two academic treatments of Leitner (KDD'16, ACL'16) both reduce recall to binary; the reference open-source implementation is binary and treats even the *demotion* rule as a config flag; and the two graded implementations I found in real code disagree with each other on both Hard and Easy. If this project defines an Again/Hard/Good/Easy → box mapping, it is inventing one, and should say so.

---

## 3. What shipped apps that display boxes/levels actually do underneath

| App | Does it display a box/level? | Underlying scheduler | Display ↔ state relationship |
|---|---|---|---|
| **Anki** | No box. Shows *states* (New/Learning/Review/Relearn) and Young/Mature. | SM-2 variant, or FSRS (DSR) since 23.10 | Young/Mature is a **pure interval threshold** (21 days). States are real. |
| **Mnemosyne** | No box; 0–5 grades | SM-2 variant | n/a |
| **WaniKani** | **Yes — 9 named SRS stages, prominently** | Stage → fixed interval table | **1:1. The stage *is* the scheduling state.** |
| **Duolingo (2012–2016)** | Yes — 4-bar strength meters | Leitner variant (one-step-back) | Projection; **documented as failing** (§4.1) |
| **Duolingo (2016–~2018)** | Yes — same 4-bar meters | Half-life regression (HLR) | Bars = predicted recall probability |
| **Duolingo (2018+)** | Crowns / levels | progress counter | **Decoupled from memory state by design** |
| **Memrise** | Levels/"words to review" | Fixed interval ladder, reset on failure | Effectively a box system |
| **Quizlet Learn** | "rounds"/progress | ML "Learning Assistant Platform" | Not documented in enough detail |

### 3.1 Anki — no box metaphor, but its *learning steps* are a box ladder

Card states, quoted verbatim from https://docs.ankiweb.net/getting-started.html **[PRIMARY]**:

- **New** — "Cards that you have downloaded or created yourself, but have never studied before."
- **Learning** — "Cards that were seen for the first time recently, and are still being learned."
- **Review** — "Cards that you have finished learning. These cards will be shown again after their delay (interval) has elapsed."
- **Relearn** — "Cards that you forgot in the review stage. These cards are returned to the relearning state to be learned again."
- **Young** — "A young card is one that has an interval of less than 21 days."
- **Mature** — "A mature card is one that has an interval of 21 days or greater."

Two important observations:

1. **These four states are a *phase* machine, not a box ladder.** A card in `Review` has a continuous real-valued interval; there is no integer level. So the box metaphor is *absent* from Anki's review phase.
2. **But Anki's learning/relearning steps *are* a box ladder** — and Anki documents exactly the Leitner shuffle over them: *"Each time you click **Good** during review, the card moves to the next step. Each time you click **Again**, the card goes back to the first step."* — https://docs.ankiweb.net/deck-options.html **[PRIMARY]**. Example steps given as `1m 10m 1d`. Graduating interval = "the number of days to wait before showing a card again, after the Good button is used on the final learning step"; easy interval is the analogue for Easy. So Anki *is* a bounded Leitner ladder during learning and a continuous interval scheduler afterwards.
3. **Young/Mature is the cleanest real-world example of a discrete display derived from an interval by threshold** (< 21 d / ≥ 21 d). If you want a precedent for "cosmetic level = f(interval)", this is it, and note that Anki chose **two** buckets and a single, documented, fixed threshold.

Anki's answer buttons, from https://docs.ankiweb.net/studying.html **[PRIMARY]**: Again (1) incorrect/not recalled; Hard (2) correct but with doubt or slow; Good (3) correct with moderate effort, "the most commonly used button"; Easy (4) correct with no mental effort.

Algorithms, from https://faqs.ankiweb.net/what-spaced-repetition-algorithm **[PRIMARY]**: *"As of Anki 23.10, Anki has two available algorithms. The first one is based on the SuperMemo 2 algorithm, and the second one is called FSRS."* FSRS tracks Retrievability, Stability, Difficulty. Anki's SM-2 "uses 4 response choices rather than 6".

### 3.2 Mnemosyne — 0–5 grades, SM-2 variant, no boxes

Mnemosyne's own principles page: the algorithm "is very similar to SM2 used in one of the early versions of SuperMemo, with some modifications that deal with early and late repetitions, and also to add a small, healthy dose of randomness to the intervals" — https://mnemosyne-proj.org/principles.php **[PRIMARY]**. Grades 0–5; 0/1 = not memorised, 2–5 = recalled with increasing ease (https://mnemosyne-proj.org/help/getting-started.php). No box display. Mnemosyne's logs are the dataset Reddy et al. binarised (§2.1).

### 3.3 WaniKani — the strongest counter-example: the box display IS the state

WaniKani shows every item's stage by name, and the stage is the scheduling state. From https://knowledge.wanikani.com/wanikani/srs-stages/ **[PRIMARY]**:

| Stage | Name | Interval to next |
|---|---|---|
| 1–4 | Apprentice 1–4 | 4 h → 8 h → 1 d → 2 d |
| 5–6 | Guru 1–2 | 1 w → 2 w |
| 7 | Master | 1 mo |
| 8 | Enlightened | 4 mo |
| 9 | Burned | — |

(Levels 1–2 use accelerated Apprentice timings: 2 h → 4 h → 8 h → 1 d.)

Promotion: *"If you get an item correct, it goes up one stage."*

Demotion is **multi-step and formula-driven** — the closest thing to a "graded" box rule I found in any shipped app:

> `new_srs_stage = current_srs_stage - (incorrect_adjustment_count * srs_penalty_factor)`
> where `incorrect_adjustment_count` is "the number of incorrect times you have answered divided by two and rounded up", and `srs_penalty_factor` is "2 if the current_srs_stage is at or above 5. Otherwise it is 1".

Note the grade here is **error count within a review**, not a self-reported Again/Hard/Good/Easy. Still binary per answer; the "grading" emerges from repeated wrong answers.

The API confirms the display and the state are the same field: `srs_stage` (0–9), `starting_srs_stage` (1–8), `ending_srs_stage` (1–9), with a `spaced_repetition_system` object whose `stages[]` carry `position` + `interval` + `interval_unit`, and positions 0 (unlocking) and 9 (burning) having null intervals — https://docs.api.wanikani.com/20170710/#reviews **[PRIMARY]**. *"If the review goes well and there are no wrong answers, we move the assignment up to the next SRS stage."*

**Takeaway for the ticket: WaniKani's stage display is trustworthy precisely because the stage is the primary state and the interval is derived from it — the opposite direction to what the ticket contemplates.**

### 3.4 Duolingo — the documented case of boxes over an interval model going wrong

See §4.1. Sequence, all from the ACL 2016 paper unless noted:

1. **At launch**: *"when it first launched, Duolingo used a variant similar to Figure 3 to manage skill meter decay and practice."* Figure 3 is captioned "The Leitner System for flashcards" with boxes **1 → 2 → 4 → 8 → 16**.
2. **Strength meters were a projection of a probability, not of a box**: *"Duolingo uses strength meters to visualize the student model… These meters represent the average probability that the student can, at any moment, correctly recall a random target word from the lessons in this skill… At four bars, the skill is 'golden' and considered fresh in the student's memory. At fewer bars, the skill has grown stale and may need practice. […] As time passes, strength meters continuously update and decay until the student practices."* **4 discrete bars over a continuous probability.**
3. **2016**: replaced by HLR (half-life regression). *"The underlying spaced repetition algorithm determined strength meter values in the skill tree… as well as the ranking of target words for practice sessions, but otherwise the two conditions were identical."* A/B test over six weeks, just under 1 million students.
4. **~2018**: crowns replaced strength bars *and the decay mechanic entirely*. Reported by TechCrunch at launch — https://techcrunch.com/2018/04/09/duolingo-adds-new-language-exercises-and-revamps-its-leveling-system — and described on the Duolingo Wiki: the crown system "replaced both the fluency badge and the skill deterioration mechanic" — https://duolingo.fandom.com/wiki/Legendary **[SECONDARY]**. In other words, **Duolingo's resolution to the "level vs. memory state" tension was to stop claiming the level means memory state at all.**

I did **not** find an official Duolingo engineering post documenting the crown scheduler; TechCrunch and the fan wiki are the best available and are secondary.

### 3.5 Memrise — an interval ladder with reset-to-first on failure

Memrise's own help centre: *"The review schedule is as follows: 4 hours > 12 hours > 24 hours > 6 days > 12 days > 48 days > 96 days > 6 months. If you get an item wrong during a review, it will be moved back to the first interval i.e. it will be up for review in 4 hours."* Max interval 180 days — https://memrise.zendesk.com/hc/en-us/articles/360015889057-How-does-the-spaced-repetition-system-work **[PRIMARY]**

This is a **binary, 8-rung, reset-to-box-1 Leitner system with explicit per-rung intervals** — functionally exactly what folklore calls "the Leitner system", published by a shipping app. Note it is **not** a doubling ladder (4 h, 12 h, 24 h, 6 d, 12 d, 48 d, 96 d, 6 mo).

### 3.6 Quizlet Learn — not documented at the needed resolution

Quizlet says Learn is "powered by the Learning Assistant Platform", uses ML over "millions of anonymous study sessions", and that its Long-Term Learning "uses a standard spaced repetition algorithm, similar to SuperMemo or Anki" — https://quizlet.com/blog/spaced-repetition-for-all-cognitive-science-meets-big-data-in-a-procrastinating-world and https://quizlet.com/content/science-behind-spaced-repetition **[PRIMARY but vague]**. **I could not verify how any Quizlet progress display maps to scheduling state.** Treat as unknown.

---

## 4. The failure mode

### 4.1 The best-documented case: Duolingo's Leitner-based strength meters, in a peer-reviewed paper

Settles & Meeder, ACL 2016, §3.2 — https://research.duolingo.com/papers/settles.acl16.pdf **[PRIMARY]**:

> "Several electronic flashcard programs use the Leitner system to schedule practice, by organizing items into 'virtual' boxes. In fact, when it first launched, Duolingo used a variant similar to Figure 3 to manage skill meter decay and practice. **The present research was motivated by the need for a more accurate model, in response to student complaints that the Leitner-based skill meters did not adequately reflect what they had learned.**"

This is close to the exact failure the ticket is worried about, stated by the vendor in a refereed venue: a discrete box display over a memory model produced a *user-visible dishonesty* strong enough to motivate replacing the scheduler. Quantitatively, in their Table, **Leitner's MAE on predicted recall was 0.235 vs HLR's 0.128** — the display was ~2× further off the truth under Leitner.

The paper also documents the *second-order* failure after they fixed it — worth noting because it is the same class of bug:

> "Several months later, active students pointed out that particular words or skills would decay rapidly, regardless of how often they practiced. Upon closer investigation, these complaints could be traced to lexeme tag features with highly negative weights in the HLR model… This implied that some feature-based overfitting had occurred, despite the L2 regularization term."

And a behavioural finding that speaks to *why* a lying meter is expensive: users optimised for the meter rather than for learning.

> "Prior to the experiment, many students claimed that they would practice instead of learning new material 'just to keep the tree gold,' but that practice sessions did not review what they thought they needed most."

### 4.2 Displayed number decreasing while the underlying state improved — Anki/FSRS

Anki forum thread "Interval goes down without any parameter changes (i.e. no optimization)" — https://forums.ankiweb.net/t/interval-goes-down-without-any-parameter-changes-i-e-no-optimization/67995 **[PRIMARY, dev/maintainer discussion]** (Dec 2025 – Jan 2026). User reported intervals *decreasing* after pressing **Good**. Maintainer explanation quoted in-thread:

> "Anki forces [good_interval >= hard_interval+1]… Due to fuzz, a 3 day interval can be 2, 3 or 4 days. In this case, it was 4 days. After 4 days elapsed, the hard interval was still 2 days, and the good interval was 3 days, which was less than 4 days."

Mechanism: **interval fuzz + minimum-gap constraints mean the *displayed* number is not a monotone function of the *underlying* memory state.** Any box number derived from the displayed interval will inherit this non-monotonicity and can go *down* after a successful review. This is a concrete, cited instance of the exact bug class.

Related and worth knowing: Anki's own manual notes Hard **increases** the interval (it is a passing grade), which users routinely read as counter-intuitive. There is an add-on, `lambdadog/passfail2` — https://github.com/lambdadog/passfail2 — whose whole premise is reducing Anki to two buttons because the 4-point scale confuses people. **[PRIMARY as an artifact; I did not read its full rationale]**

### 4.3 Is there a principled way to derive a box number from an interval or from stability?

Yes — I found **three** defensible derivations, and they disagree on the functional form. That disagreement is itself the finding.

**(a) Logarithmic, from Duolingo's HLR (published).** Leitner as an HLR special case gives half-life `h = 2^(x⊕ − x⊖)`; identifying box `b` with the net streak `x⊕ − x⊖` gives

```
interval(b) ∝ 2^b        ⟺        b = log2(half_life)
```
— https://research.duolingo.com/papers/settles.acl16.pdf, Appendix A.2. **[PRIMARY]**

**(b) Logarithmic with a tunable base, from Woźniak's Normalized Leitner.** "first interval is set to Int1, and successive intervals are set to Int1*power(E-Factor, repetition)" gives

```
b = log_EF(interval / Int1)      (Int1 = 1, EF = 2 ⇒ boxes at 1, 2, 4, 8, 16 days)
```
— https://supermemo.guru/wiki/Leitner_system. Woźniak also states the conceptual mapping directly: **"Well known cards are shunted to boxes corresponding with higher memory stability."** So box ≈ a discretisation of **stability**, not of interval — which is the more robust choice, since stability is monotone under successful review whereas the scheduled interval is not (§4.2). **[PRIMARY as Woźniak's definition]**

**(c) Linear, from Reddy et al.'s empirically-fitted memory model.** They tested three candidates for memory strength *s* on 859,591 Mnemosyne interactions (2,742 users, 88,892 items) and found:

> "**Leitner position vs. number of reviews:** Setting the memory strength s to be equal to the Leitner deck position q_ij performs better than setting it to be proportional to the number of past reviews n_ij, which in turn is better than using a constant s."

giving `P[recall] = exp(−θ · d_i / q_i)`. Solving for the interval at a fixed target retention p*:

```
d = q · (−ln p*) / θ        ⟹        interval ∝ box  (LINEAR)
⟺  box = θ · interval / (−ln p*)
```
— https://www.cs.cornell.edu/~tj/publications/reddy_etal_16c.pdf. **[PRIMARY]**

So the only *empirically fitted* box↔interval relationship I found is **linear**, while the two hand-picked/normative ones are **exponential**. The same paper also found deck position is a *better* memory-strength proxy than raw review count, which is a genuine argument that a box number carries real information — just not the information the doubling folklore assumes.

**(d) The interval-threshold approach, as actually shipped.** Anki's Young/Mature split at a 21-day interval — https://docs.ankiweb.net/getting-started.html. Two buckets, one documented threshold. **[PRIMARY]**

### 4.4 What I could NOT find

- **No issue thread or dev discussion I could cite where a *box/level* display specifically was called "a lie" or "misleading" in an open-source SRS.** I searched GitHub (repo + code search via `gh api search/*`), Anki forums, and general web. The Duolingo ACL paper (§4.1) is the only first-party documentation of the box-display-is-wrong complaint that I could verify. **Absence of evidence here may just reflect that few open-source apps ship a box UI over an interval scheduler** — the largest Leitner repos on GitHub are tiny (see §5.3) and xanki deliberately hides its box (§2.4c).
- **No documented case of "card shown in a high box that is actually due immediately."** The closest verified thing is §4.2 (displayed interval decreasing after a pass). The mechanism is plausible for a derived box number but I did not find it reported for a box UI specifically.

---

## 5. Box-count and interval conventions in practice — and where the numbers come from

### 5.1 The doubling table 1-2-4-8-16 is **Mace (1932), not Leitner**

Woźniak, on his Normalized Leitner boxes at Int1=1, EF=2: *"This would associate boxes with intervals: 1, 2, 4, 8, and 16 days (i.e. **as in the schedule suggested in C.A. Mace book of 1932**)."* — https://supermemo.guru/wiki/Leitner_system **[PRIMARY as attribution]**

Corroborated by SuperMemo's history article: *"In 1932, C. A. Mace hinted on the efficient learning methods in his book 'The psychology of study'. He mentioned 'active rehearsal' and 'repetitive revisions' that should be spaced in gradually increasing intervals, roughly **'intervals of one day, two days, four days, eight days, and so on'**."* — https://www.supermemo.com/en/blog/the-true-history-of-spaced-repetition **[SECONDARY re: Mace]**

Note the same page **does not mention Leitner at all**. Duolingo's Figure 3 ("The Leitner System for flashcards") uses exactly 1-2-4-8-16 — https://research.duolingo.com/papers/settles.acl16.pdf — which means the canonical academic *illustration* of "the Leitner system" is in fact Mace's 1932 schedule. **This is the clearest documented case of Leitner folklore: an interval table universally attributed to Leitner that traces to a different author, 40 years earlier.**

### 5.2 Box counts and intervals actually used, with provenance

| Boxes | Intervals | Source | Provenance quality |
|---|---|---|---|
| 5 partitions | 1, 2, 5, 8, 14 **cm** (capacity, not days) | https://en.wikipedia.org/wiki/Leitner_system | closest to Leitner; **uncited** in the article |
| 3 | 1 d, every 2nd d, every 4th d — with proportional sampling (all / ½ / ¼) | https://de.wikipedia.org/wiki/Lernkartei | uncited |
| 3 | 1 d, 3 d, 5 d | https://en.wikipedia.org/wiki/Leitner_system, "Three boxes" section | **uncited**; section has no refs at all |
| 12 (10 rotating + Current + Retired) | session-number scheduling, "identical to a 5-box Leitner system" | https://en.wikipedia.org/wiki/Leitner_system, "Proficiency levels" | **uncited**; unattributed scheme |
| 5 | 1, 2, 4, 8, 16 d | Mace 1932 via Woźniak & Duolingo Fig. 3 | **traceable to Mace, not Leitner** |
| 3 (default) | **1, 2, 7 d** | `open-spaced-repetition/leitner-box` code | PRIMARY code, but numbers are the author's choice |
| 5 | **1, 3, 7, 30, 180 d** | `nickhnsn/facharbeit-spaced-repetition`, `LeitnerAlgorithm.java` | PRIMARY code, author's choice |
| 5 | **1, 3, 7, 14, 30 d** | `yro7/panglot-public`, `leitner.rs` | PRIMARY code, author's choice |
| 5 (boxes 2–5 for review) | **1, 3, 7, 21 d** + steps 1 m/10 m | `SouichiroTsujimoto/xanki` spec | PRIMARY spec, author's choice |
| 8 | **4 h, 12 h, 24 h, 6 d, 12 d, 48 d, 96 d, 6 mo** (cap 180 d) | https://memrise.zendesk.com/hc/en-us/articles/360015889057-How-does-the-spaced-repetition-system-work | PRIMARY, shipped |
| 9 | **4 h, 8 h, 1 d, 2 d, 1 w, 2 w, 1 mo, 4 mo, burned** | https://knowledge.wanikani.com/wanikani/srs-stages/ | PRIMARY, shipped |
| n = 5 (in experiments) | rate-based μ_k, no fixed intervals | Reddy et al. KDD'16 | PRIMARY, peer-reviewed |
| 2 (display only) | threshold at 21 d | https://docs.ankiweb.net/getting-started.html | PRIMARY, shipped |

**Conclusion for Q5: there is no canonical box count and no canonical interval table.** 3, 5, 8, 9, and 12 all appear. Every day-interval table I found is either (i) an implementer's arbitrary choice, (ii) Mace's 1932 doubling schedule mis-attributed to Leitner, or (iii) uncited Wikipedia prose. **5 boxes** is the most defensible count if you want a nod to the original, on the strength of the 1/2/5/8/14 cm partition claim and Reddy et al. taking n = 5 "in experiments"; note that Woźniak-referenced German literature also markets a "5-Fächer-Lernbox" (Fenske, *Bio-logisch lernen mit der 5-Fächer-Lernbox*, AOL-Verlag 2002, listed at https://de.wikipedia.org/wiki/Lernkartei).

### 5.3 Scale check on the open-source Leitner ecosystem

Via `gh api search/repositories q='leitner spaced repetition stars:>20'`: only **two** repos clear 20 stars — `rddy/leitnerq` (32★, the KDD paper's code) and `nickhnsn/facharbeit-spaced-repetition` (29★). A `q='leitner box flashcards'` search returns a long tail of 0–8★ projects (`RezaGooner/Leitner-Box` 8★, `mohamad-mgn/Leitner-Box` 3★, `Tobi-De/leerming` 3★, `FarzamTP/Leitner` 2★, `paulreece/Leitner-Flashcards` "4 boxes", `riccardo-montanari/flashcard-leitner` "5 box", etc.). **There is no widely-adopted open-source Leitner app whose scheduling decisions carry authority.** Anything you copy from that tail is one person's guess.

---

## 6. Unverified / open questions

1. **Leitner's book itself — unread.** I could not obtain *So lernt man lernen* in any edition. Everything in §1 is second-hand. In particular I could **not** verify: (a) whether the 1/2/5/8/14 cm partition sizes are actually in the book; (b) whether Leitner states *any* day-interval; (c) whether Leitner's demotion rule is "back one" or "back to box 1"; (d) whether he even says five boxes. **If this decision matters, someone should read the book.** ISBN 978-3-451-05060-2 (18th ed., Herder 2011).
2. **Publication year is genuinely disputed in citable literature**: 1970 (Reddy et al. prose), 1972 (both Wikipedias, Settles & Meeder), 1974 (Reddy et al. reference list). Unresolved.
3. **Duolingo strength-bar → probability threshold mapping: NOT documented.** The ACL paper says the bars "represent the average probability that the student can… correctly recall a random target word" and that four bars = golden. It gives **no formula or thresholds** for how a probability becomes 1/2/3/4 bars. ⚠️ **A first automated read of the PDF produced a formula `strength = min(1, −0.5/ln(p)·c)`; I re-extracted the PDF text with `pdftotext` and confirmed that string does not appear in the paper. It was a hallucination. Do not use it.**
4. **Duolingo's post-2018 crown scheduler is undocumented by Duolingo.** I have only TechCrunch and a fan wiki for "crowns replaced the decay mechanic". No official engineering description found. Also flagged (fan wiki, unverified) that the crown system itself was changed again around Oct–Nov 2022.
5. **Quizlet Learn:** cannot verify any mapping from its progress display to scheduling state. Their published descriptions are marketing-level.
6. **No cited example found of "high box, actually due now."** §4.2 gives the mechanism (fuzz + minimum-gap constraints break monotonicity of the displayed interval) but I found no report of it manifesting through a box UI. Consider this a predicted, not observed, failure.
7. **The linear vs. exponential box↔interval discrepancy (§4.3) is unreconciled.** Reddy et al.'s empirical fit says interval ∝ box; Mace/Woźniak/Duolingo-Fig-3 folklore says interval ∝ 2^box. I found no paper that addresses this contradiction. Reddy et al.'s fit is on Mnemosyne (SM-2) logs, where deck position was *reconstructed*, not native — that may be a confound, but I could not confirm how they derived `q_ij` for a system with no decks.
8. **`supermemo.guru` returns HTTP 403 to plain fetchers.** All Woźniak quotes above were retrieved with `curl -A "Mozilla/5.0 …"`. Page last edited 2018-11-21; permalink oldid=12663.
9. **GitHub code search coverage is partial** (it indexes a subset of repos and the default-branch only). Some of the `search/code` hits for graded-Leitner specs are in small, possibly AI-assisted repos; I read the two most substantive (`panglot-public`, `xanki`) directly and quote their actual code/spec, but their *authority* is nil — they are evidence that graded Leitner is invented per-project, not evidence of a convention.
