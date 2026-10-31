package dev.cairn.app

import android.text.BidiFormatter
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.design.Cairn
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnRule
import dev.cairn.design.CairnText
import dev.cairn.design.CairnTextAction
import dev.cairn.design.Family
import dev.cairn.design.Tier
import dev.cairn.design.gap
import dev.cairn.design.isRightToLeft
import dev.cairn.design.reachLineBelow
import dev.cairn.ffi.DeckEffects
import dev.cairn.ffi.DeckPath
import dev.cairn.ffi.FilePreview
import dev.cairn.ffi.FileRefusal
import dev.cairn.ffi.ImportGate
import dev.cairn.ffi.ImportPlanFacts

/** The import screen over its model: the preview, or the failure page when the file cannot be read. */
@Composable
fun ImportScreen(model: ImportViewModel, onLeave: () -> Unit, onImport: () -> Unit = {}) {
    val state by model.state.collectAsStateWithLifecycle()
    when (val s = state) {
        // Nothing is held: the caller has already left, or is about to (a killed process is not restored).
        is ImportScreenState.Empty -> CairnPage {}
        // A file replacing another opens at its page's top (#241); a plan read again keeps the place.
        is ImportScreenState.Preview ->
            key(s.opened) { ImportPage(s.name, s.preview, onImport = onImport, onLeave = onLeave) }
        is ImportScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("import-failed")) }
    }
}

/**
 * The import preview (ADR-0022 §3, ADR-0041 §1-§3): the file's name as it arrived, its own claims in
 * weak text above a hairline, each deck in its own direction with its effects under it, and the line
 * that is always there. A refusal is its sentence alone. The gate is pinned on the reach line outside
 * the scroll, and the page above it opens at its top.
 */
@Composable
fun ImportPage(name: String?, preview: FilePreview, onImport: () -> Unit, onLeave: () -> Unit) {
    CairnPage(Modifier.testTag("import-screen")) {
        BoxWithConstraints {
            val page: Dp = maxHeight
            Column(Modifier.fillMaxWidth()) {
                Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(top = gap(1))) {
                    CairnText(
                        name ?: stringResource(R.string.import_untitled),
                        Modifier.fillMaxWidth().testTag("import-heading"),
                        tier = Tier.Heading,
                    )
                    when (preview) {
                        is FilePreview.Refused -> {
                            Spacer(Modifier.height(gap(2)))
                            Sentence(refusalSentence(preview.refusal), "import-refusal")
                        }
                        is FilePreview.Plan -> PlanBody(preview.plan)
                    }
                    Spacer(Modifier.height(gap(2)))
                }
                Spacer(Modifier.height(gap(2)))
                Gate(preview.gate(), onImport, onLeave)
                Spacer(Modifier.height(reachLineBelow(page)))
            }
        }
    }
}

private fun FilePreview.gate(): ImportGate = when (this) {
    is FilePreview.Refused -> gate
    is FilePreview.Plan -> plan.gate
}

/** *Import* as the primary with *Cancel* beside it as a text action, or a lone *Close* (ADR-0041 §3). */
@Composable
private fun Gate(gate: ImportGate, onImport: () -> Unit, onLeave: () -> Unit) {
    when (gate) {
        ImportGate.DISMISS ->
            CairnControl(stringResource(R.string.import_close), onLeave, Modifier.fillMaxWidth().testTag("gate-close"), primary = true)
        ImportGate.DECIDE -> Row(verticalAlignment = Alignment.CenterVertically) {
            CairnControl(
                stringResource(R.string.import_import),
                onImport,
                Modifier.weight(1f).testTag("gate-import"),
                primary = true,
            )
            Spacer(Modifier.width(gap(1)))
            CairnTextAction(stringResource(R.string.import_cancel), onLeave, Modifier.testTag("gate-cancel"))
        }
    }
}

@Composable
private fun PlanBody(plan: ImportPlanFacts) {
    Header(plan)
    plan.decks.forEachIndexed { i, deck ->
        if (i > 0) Spacer(Modifier.height(gap(3)))
        DeckBlock(i, deck)
    }

    // The file-level lines, after every deck: the kinds it adopts and the held decks its moves leave
    // empty. Both are about the collection rather than about one deck in the file.
    val fileLines = buildList {
        if (plan.adoptedKinds.isNotEmpty()) add(stringResource(R.string.import_adds_kinds, plan.adoptedKinds.joinToString(", ") { wrapped(it) }))
        plan.emptiedDecks.forEach { add(stringResource(R.string.import_left_empty, wrapped(it))) }
    }
    if (fileLines.isNotEmpty()) {
        Spacer(Modifier.height(gap(2)))
        Lines(fileLines)
    }
    // Always present, even when it is the only line (ADR-0022 §3).
    Spacer(Modifier.height(gap(2)))
    Sentence(stringResource(R.string.import_history_untouched), "import-line")
}

/**
 * The file's own claims, set apart from the effects (ADR-0022 §7, ADR-0041 §1): in weak text above a
 * hairline, `gap(2)` either side. A file with no header draws no rule — a hairline under nothing divides nothing.
 */
@Composable
private fun Header(plan: ImportPlanFacts) {
    val byline = listOfNotNull(
        plan.author.takeIf { it.isNotEmpty() }?.let { stringResource(R.string.import_by, wrapped(it)) },
        plan.licence.takeIf { it.isNotEmpty() }?.let { wrapped(it) },
    ).joinToString(" · ")
    val claims = listOf(byline, plan.description.takeIf { it.isNotEmpty() }?.let { wrapped(it) } ?: "").filter { it.isNotEmpty() }
    if (claims.isEmpty()) {
        Spacer(Modifier.height(gap(3)))
        return
    }
    Spacer(Modifier.height(gap(1)))
    claims.forEachIndexed { i, claim ->
        if (i > 0) Spacer(Modifier.height(gap(1)))
        CairnText(claim, Modifier.fillMaxWidth().testTag("import-claim-$i"), color = Cairn.colors.weak)
    }
    Spacer(Modifier.height(gap(2)))
    CairnRule(Modifier.testTag("import-header-rule"))
    Spacer(Modifier.height(gap(2)))
}

/**
 * One deck: its name in the bold face and the path it takes as a caption, both aligned by the name's
 * own direction, and its effects indented one step beneath, keeping the interface's edge (ADR-0041 §2).
 */
@Composable
private fun DeckBlock(index: Int, deck: DeckEffects) {
    CairnText(deck.name, Modifier.fillMaxWidth().testTag("import-deck-$index"), family = Family.Bold)
    val caption = when {
        deck.noChange -> R.string.import_caption_held
        deck.path == DeckPath.UPDATE -> R.string.import_caption_update
        else -> R.string.import_caption_create
    }
    // The caption follows the name's direction, not its own words': it is the name's footnote.
    CairnText(
        stringResource(caption),
        Modifier.fillMaxWidth().testTag("import-caption-$index"),
        tier = Tier.Small,
        color = Cairn.colors.weak,
        direction = if (isRightToLeft(deck.name)) TextDirection.Rtl else TextDirection.Ltr,
    )
    Spacer(Modifier.height(gap(1)))
    Column(Modifier.padding(start = gap(2))) { Lines(effectLines(deck)) }
}

/** A deck's effect lines, in ADR-0022 §3's order. A line that does not apply is absent, never shown as zero. */
@Composable
private fun effectLines(deck: DeckEffects): List<String> {
    if (deck.noChange) return listOf(stringResource(R.string.import_nothing_changes))
    return buildList {
        val new = deck.newNotes.toLong()
        val held = deck.alreadyYours.toLong()
        when {
            new == 0L && held == 0L -> {}
            new == 0L -> add(stringResource(R.string.import_already_yours, grouped(held)))
            held == 0L -> add(counted(R.plurals.import_new_notes, new))
            else -> add(stringResource(R.string.import_new_and_already_yours, counted(R.plurals.import_new_notes, new), grouped(held)))
        }
        for (moving in deck.movingIn) {
            // *Unfiled* is the note list's own name for notes in no deck (ADR-0039 §5).
            val from = moving.from?.let { wrapped(it) } ?: stringResource(R.string.notes_unfiled)
            add(stringResource(R.string.import_moving_in, counted(R.plurals.import_notes, moving.count.toLong()), from))
        }
        if (deck.deleted > 0u) add(stringResource(R.string.import_deleted, grouped(deck.deleted.toLong())))
        deck.renamedFrom?.let { add(stringResource(R.string.import_renaming, wrapped(it), wrapped(deck.name))) }
        if (deck.revisionConflict) add(stringResource(R.string.import_revision_conflict))
    }
}

@Composable
private fun refusalSentence(refusal: FileRefusal): String = when (refusal) {
    is FileRefusal.Unreadable -> stringResource(R.string.import_refusal_unreadable)
    is FileRefusal.NeedsNewerVersion -> stringResource(R.string.import_refusal_newer)
    is FileRefusal.CollectionArchive -> stringResource(R.string.import_refusal_archive)
    is FileRefusal.NotADeck -> stringResource(R.string.import_refusal_not_a_deck)
    // Named as older, never as damaged (ADR-0022 §4), and it names the held deck.
    is FileRefusal.OlderCopy -> stringResource(R.string.import_refusal_older, wrapped(refusal.deck))
}

/** Body lines, one unit apart, each in the interface's own direction so a name cannot turn the line round. */
@Composable
private fun Lines(lines: List<String>) {
    lines.forEachIndexed { i, line ->
        if (i > 0) Spacer(Modifier.height(gap(1)))
        Sentence(line, "import-line")
    }
}

/** One of the application's own sentences: it keeps the interface's edge whatever name it carries. */
@Composable
private fun Sentence(text: String, tag: String) {
    val edge = if (LocalLayoutDirection.current == LayoutDirection.Rtl) TextDirection.Rtl else TextDirection.Ltr
    CairnText(text, Modifier.fillMaxWidth().testTag(tag), direction = edge)
}

/** A name inside a sentence is isolated, as the note list's delete question does. */
private fun wrapped(name: String): String = BidiFormatter.getInstance().unicodeWrap(name)

private fun grouped(n: Long): String = String.format(java.util.Locale.ENGLISH, "%,d", n)

@Composable
private fun counted(plural: Int, n: Long): String =
    pluralStringResource(plural, n.coerceAtMost(Int.MAX_VALUE.toLong()).toInt(), grouped(n))
