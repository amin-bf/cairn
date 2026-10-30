package dev.cairn.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.cairn.design.Cairn
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText
import dev.cairn.design.CairnTextAction
import dev.cairn.design.Mark
import dev.cairn.design.Tier
import dev.cairn.design.tokens.Spacing
import dev.cairn.ffi.ReviewFacts
import dev.cairn.ffi.ReviewStateKind

private fun gap(units: Int): Dp = (Spacing.UNIT * units).dp

/**
 * Review's entrance (ADR-0006 §7, §8; ADR-0034 §2, §3): the heading, one worded statement of the
 * state, and — where there is work — the primary way in and the shorter sittings beside it. The
 * words are string resources; which one is drawn, and whether the shorter line is, is the facts'
 * alone (ADR-0046 §1, §4). [onStart] is told how many gradings the person chose.
 */
@Composable
fun ReviewEntrance(facts: ReviewFacts, onStart: (Int) -> Unit, modifier: Modifier = Modifier) {
    CairnPage(modifier) {
        Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(top = gap(2))) {
            CairnText(stringResource(R.string.review_heading), tier = Tier.Heading)
            Spacer(Modifier.height(gap(2)))
            when (val state = facts.state) {
                is ReviewStateKind.Empty -> Statement(stringResource(R.string.review_empty), "state-empty")
                is ReviewStateKind.CaughtUp -> CaughtUp()
                is ReviewStateKind.NewDeck -> {
                    Statement(stringResource(R.string.review_new_deck), "state-new-deck")
                    Ways(facts, onStart)
                }
                is ReviewStateKind.NewOnly -> {
                    Statement(
                        pluralStringResource(R.plurals.review_new_only, state.new.toInt(), state.new.toInt()),
                        "state-new-only",
                    )
                    Ways(facts, onStart)
                }
                is ReviewStateKind.Due -> {
                    val words = when {
                        state.backlog -> stringResource(R.string.review_due_backlog)
                        state.new > 0u -> stringResource(R.string.review_due_plus_new, state.due.toInt(), state.new.toInt())
                        else -> stringResource(R.string.review_due, state.due.toInt())
                    }
                    Statement(words, "state-due")
                    Ways(facts, onStart)
                }
            }
        }
    }
}

@Composable
private fun Statement(words: String, tag: String) {
    CairnText(words, Modifier.testTag(tag))
}

/** The floor gets the screen (ADR-0034 §3): the mark, then the answer, then its footnote, centred. */
@Composable
private fun CaughtUp() {
    Column(
        Modifier.fillMaxWidth().padding(top = gap(8)).testTag("state-caught-up"),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Mark(Modifier.testTag("caught-up-mark"), color = Cairn.colors.weak)
        Spacer(Modifier.height(gap(8)))
        CairnText(stringResource(R.string.review_caught_up), tier = Tier.Display)
        Spacer(Modifier.height(gap(2)))
        CairnText(stringResource(R.string.review_caught_up_note), tier = Tier.Small, color = Cairn.colors.weak)
    }
}

/** One primary way in, the shorter sittings a quiet second line, only when any is shorter (ADR-0034 §2). */
@Composable
private fun Ways(facts: ReviewFacts, onStart: (Int) -> Unit) {
    val available = facts.available.toInt()
    Spacer(Modifier.height(gap(3)))
    CairnControl(
        if (available == 1) stringResource(R.string.review_start_one) else stringResource(R.string.review_start_all, available),
        onClick = { onStart(available) },
        primary = true,
        modifier = Modifier.fillMaxWidth().testTag("start-all"),
    )
    if (facts.shorter.isNotEmpty()) {
        Spacer(Modifier.height(gap(2)))
        Row(
            Modifier.testTag("shorter-sittings"),
            horizontalArrangement = Arrangement.spacedBy(gap(1)),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CairnText(stringResource(R.string.review_shorter), tier = Tier.Small, color = Cairn.colors.weak)
            for (count in facts.shorter) {
                CairnTextAction(count.toString(), onClick = { onStart(count.toInt()) }, modifier = Modifier.testTag("shorter-$count"))
            }
        }
    }
}
