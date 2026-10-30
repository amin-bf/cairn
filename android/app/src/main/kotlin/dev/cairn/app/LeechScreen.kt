package dev.cairn.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.design.Cairn
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnRuns
import dev.cairn.design.CairnText
import dev.cairn.design.CairnWell
import dev.cairn.design.Tier
import dev.cairn.design.gap
import dev.cairn.design.tokens.Controls
import dev.cairn.design.tokens.Frame
import dev.cairn.design.tokens.Spacing
import dev.cairn.ffi.LeechCost
import dev.cairn.ffi.LeechListFacts
import dev.cairn.ffi.LeechRowFact

/** The gap between two rows: four units, against the one that separates the parts inside a row (#156). */
private val BetweenRows = gap(4)

/** The leech screen over its model: the list, or the failure page when the collection cannot be read. */
@Composable
fun LeechScreen(model: LeechViewModel, onEdit: (noteId: String) -> Unit, onBack: () -> Unit) {
    val state by model.state.collectAsStateWithLifecycle()
    when (val s = state) {
        is LeechScreenState.Loading -> CairnPage {}
        is LeechScreenState.Ready -> LeechList(
            s.facts,
            s.refusal,
            onEdit = { onEdit(it.noteId) },
            onSuspend = model::suspend,
            onUnsuspend = model::unsuspend,
            onBack = onBack,
        )
        is LeechScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("leech-failed")) }
    }
}

/**
 * The leech screen (ADR-0010 §4 §7 §8, ADR-0043, ADR-0050): the ranked leeches worst first, one well
 * per card, then the permanent suspended section; or one sentence when there is neither. *Back to
 * review* is pinned outside the scroll, on the reach line when the page has room for it, at the
 * ordinary weight — a screen holding wells takes no primary (ADR-0034 §2).
 */
@Composable
fun LeechList(
    facts: LeechListFacts,
    refusal: LeechRefusal?,
    onEdit: (LeechRowFact) -> Unit,
    onSuspend: (LeechRowFact) -> Unit,
    onUnsuspend: (LeechRowFact) -> Unit,
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    CairnPage(modifier.testTag("leech-screen")) {
        BoxWithConstraints {
            val page: Dp = maxHeight
            Column(Modifier.fillMaxWidth()) {
                Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(top = gap(2))) {
                    CairnText(stringResource(R.string.review_heading), tier = Tier.Heading)
                    Spacer(Modifier.height(gap(2)))
                    if (facts.active.isEmpty() && facts.suspended.isEmpty()) {
                        CairnText(stringResource(R.string.leeches_empty), Modifier.testTag("leeches-empty"))
                    }
                    if (facts.active.isNotEmpty()) {
                        CairnText(stringResource(R.string.leeches_statement), Modifier.testTag("leeches-statement"))
                        Spacer(Modifier.height(gap(1)))
                        Rows(facts.active, refusal, onEdit, onSuspend, onUnsuspend)
                    }
                    if (facts.suspended.isNotEmpty()) {
                        if (facts.active.isNotEmpty()) Spacer(Modifier.height(gap(3)))
                        CairnText(stringResource(R.string.leeches_suspended_statement), Modifier.testTag("suspended-statement"))
                        Spacer(Modifier.height(gap(1)))
                        Rows(facts.suspended, refusal, onEdit, onSuspend, onUnsuspend)
                    }
                    Spacer(Modifier.height(gap(2)))
                }
                // The reach line when it fits in a third of the page, as the note list's does.
                val full = (Frame.REACH_LINE + Controls.HEIGHT + Spacing.UNIT).dp
                Spacer(Modifier.height(gap(2)))
                CairnControl(
                    stringResource(R.string.leeches_back),
                    onClick = onBack,
                    modifier = Modifier.fillMaxWidth().testTag("back-to-review"),
                )
                Spacer(Modifier.height(if (full <= page / 3) Frame.REACH_LINE.dp else gap(2)))
            }
        }
    }
}

@Composable
private fun Rows(
    rows: List<LeechRowFact>,
    refusal: LeechRefusal?,
    onEdit: (LeechRowFact) -> Unit,
    onSuspend: (LeechRowFact) -> Unit,
    onUnsuspend: (LeechRowFact) -> Unit,
) {
    for ((i, row) in rows.withIndex()) {
        if (i > 0) Spacer(Modifier.height(BetweenRows))
        LeechRow(row, refusal?.takeIf { it.noteId == row.noteId && it.slot == row.slot }, onEdit, onSuspend, onUnsuspend)
    }
}

/** One card: its prompt (and cost) in a well, and what may be done to it underneath, in words. */
@Composable
private fun LeechRow(
    row: LeechRowFact,
    refusal: LeechRefusal?,
    onEdit: (LeechRowFact) -> Unit,
    onSuspend: (LeechRowFact) -> Unit,
    onUnsuspend: (LeechRowFact) -> Unit,
) {
    val key = "${row.noteId}-${row.slot}"
    Column(Modifier.fillMaxWidth().testTag("leech-row-$key")) {
        CairnWell {
            if (row.generated) {
                CairnRuns(row.prompt.toCardRuns(), row.rtl, Modifier.testTag("leech-prompt-$key"))
            } else {
                CairnText(stringResource(R.string.leeches_not_generated), Modifier.testTag("leech-prompt-$key"))
            }
            row.cost?.let {
                Spacer(Modifier.height(gap(1)))
                CairnText(costLine(it), Modifier.testTag("leech-cost-$key"), tier = Tier.Small, color = Cairn.colors.weak)
            }
        }
        Spacer(Modifier.height(gap(1)))
        Row(horizontalArrangement = Arrangement.spacedBy(gap(1))) {
            CairnControl(stringResource(R.string.leeches_edit), onClick = { onEdit(row) }, modifier = Modifier.testTag("leech-edit-$key"))
            if (row.cost != null) {
                CairnControl(stringResource(R.string.leeches_suspend), onClick = { onSuspend(row) }, modifier = Modifier.testTag("leech-suspend-$key"))
            } else {
                CairnControl(stringResource(R.string.leeches_unsuspend), onClick = { onUnsuspend(row) }, modifier = Modifier.testTag("leech-unsuspend-$key"))
            }
        }
        if (refusal != null) {
            Spacer(Modifier.height(gap(1)))
            CairnText(
                stringResource(if (refusal.suspending) R.string.leeches_suspend_refused else R.string.leeches_unsuspend_refused),
                Modifier.testTag("leech-refused-$key"),
                tier = Tier.Small,
            )
        }
    }
}

/** *{d} bad days · {r} reviews · {time}* — the time said as the egui build says it. */
@Composable
private fun costLine(cost: LeechCost): String {
    val time = when (cost.answerMinutes) {
        0u -> stringResource(R.string.leeches_time_under_a_minute)
        1u -> stringResource(R.string.leeches_time_one_minute)
        else -> stringResource(R.string.leeches_time_minutes, cost.answerMinutes.toInt())
    }
    return stringResource(R.string.leeches_cost, cost.failureDays.toInt(), cost.reviews.toInt(), time)
}
