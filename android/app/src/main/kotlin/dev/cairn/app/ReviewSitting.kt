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
import androidx.compose.runtime.key
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.cairn.design.Cairn
import dev.cairn.design.CairnCard
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText
import dev.cairn.design.CardRun
import dev.cairn.design.ReachColumn
import dev.cairn.design.Tier
import dev.cairn.design.tokens.Spacing
import dev.cairn.ffi.Grade
import dev.cairn.ffi.Run
import dev.cairn.ffi.RunStyle
import dev.cairn.ffi.ShownCard

private fun gap(units: Int): Dp = (Spacing.UNIT * units).dp

private fun List<Run>.toCardRuns(): List<CardRun> = map {
    CardRun(
        it.text,
        bold = it.style == RunStyle.BOLD || it.style == RunStyle.BOLD_ITALIC,
        italic = it.style == RunStyle.ITALIC || it.style == RunStyle.BOLD_ITALIC,
        code = it.style == RunStyle.CODE,
    )
}

/**
 * A sitting (ADR-0006, ADR-0033 to ADR-0035, ADR-0037): the heading, the check-in when it has fallen
 * due, the progress line, the card, and — only once the card is revealed — the grades, their cluster
 * sitting on the reach line. Whether a grade is allowed, when the check-in is due and when the
 * sitting is over are the sitting's to say; this draws what it reports (ADR-0046 §1, §3).
 */
@Composable
fun ReviewSitting(
    chosen: Int,
    graded: Int,
    card: ShownCard,
    checkpoint: Boolean,
    onReveal: () -> Unit,
    onGrade: (Grade) -> Unit,
    onKeepGoing: () -> Unit,
    onFinish: () -> Unit,
    modifier: Modifier = Modifier,
) {
    CairnPage(modifier) {
        BoxWithConstraints(Modifier.testTag("page-room")) {
            val room = maxHeight
            Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState())) {
                ReachColumn(
                    room = room,
                    floor = gap(3),
                    top = {
                        Column(Modifier.padding(top = gap(2))) {
                            CairnText(stringResource(R.string.review_heading), tier = Tier.Heading)
                            Spacer(Modifier.height(gap(2)))
                            if (checkpoint) {
                                Checkpoint(onKeepGoing, onFinish)
                                Spacer(Modifier.height(gap(3)))
                            }
                            CairnText(stringResource(R.string.review_progress, graded, chosen), Modifier.testTag("progress"))
                            Spacer(Modifier.height(gap(2)))
                            // A new card is a new composition: its reveal starts shut, and the next card
                            // appears without a fade (ADR-0037 §4).
                            key(card.noteId, card.ordinal, graded) {
                                CairnCard(
                                    prompt = card.prompt.toCardRuns(),
                                    answer = card.answer.toCardRuns(),
                                    badge = if (card.isNew) stringResource(R.string.review_badge_new) else stringResource(R.string.review_badge_box, card.boxNumber.toInt()),
                                    rtl = card.rtl,
                                    revealed = card.revealed,
                                    onReveal = onReveal,
                                )
                            }
                        }
                    },
                    reach = { if (card.revealed) Grades(card, onGrade) },
                )
            }
        }
    }
}

/** The ten-minute check-in: one row above the card, which stays on screen and gradeable (ADR-0034 §4). */
@Composable
private fun Checkpoint(onKeepGoing: () -> Unit, onFinish: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().testTag("checkpoint"),
        horizontalArrangement = Arrangement.spacedBy(gap(2)),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CairnText(stringResource(R.string.review_checkpoint), tier = Tier.Small, color = Cairn.colors.weak)
        CairnControl(stringResource(R.string.review_finish_here), onFinish, Modifier.testTag("finish-here"))
        CairnControl(stringResource(R.string.review_keep_going), onKeepGoing, Modifier.testTag("keep-going"))
    }
}

/**
 * The touch layout (ADR-0035 §3): *Forgot* full width, held apart by three units, then *Barely*,
 * *Good* and *Easy* stacked a unit apart, each with its next interval in days beside it.
 */
@Composable
private fun Grades(card: ShownCard, onGrade: (Grade) -> Unit) {
    Column(Modifier.fillMaxWidth().testTag("grades")) {
        GradeControl(R.string.review_grade_forgot, card.previews.forgot, Grade.FORGOT, "grade-forgot", onGrade)
        Spacer(Modifier.height(gap(3)))
        GradeControl(R.string.review_grade_barely, card.previews.barely, Grade.BARELY, "grade-barely", onGrade)
        Spacer(Modifier.height(gap(1)))
        GradeControl(R.string.review_grade_good, card.previews.good, Grade.GOOD, "grade-good", onGrade)
        Spacer(Modifier.height(gap(1)))
        GradeControl(R.string.review_grade_easy, card.previews.easy, Grade.EASY, "grade-easy", onGrade)
    }
}

@Composable
private fun GradeControl(name: Int, days: UInt, grade: Grade, tag: String, onGrade: (Grade) -> Unit) {
    CairnControl(
        stringResource(name),
        onClick = { onGrade(grade) },
        modifier = Modifier.fillMaxWidth().testTag(tag),
        aside = stringResource(R.string.review_interval_days, days.toInt()),
    )
}
