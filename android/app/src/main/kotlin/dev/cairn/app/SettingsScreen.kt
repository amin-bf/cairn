package dev.cairn.app

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.design.CairnDestinationsRow
import dev.cairn.design.CairnFieldLabel
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnProgress
import dev.cairn.design.CairnRule
import dev.cairn.design.CairnText
import dev.cairn.design.CairnTextField
import dev.cairn.design.Destination
import dev.cairn.design.Family
import dev.cairn.design.Tier
import dev.cairn.design.gap
import dev.cairn.ffi.ThemeChoice

/** Settings over its model: the page, or the failure page when the collection cannot be read. */
@Composable
fun SettingsScreen(model: SettingsViewModel) {
    val state by model.state.collectAsStateWithLifecycle()
    when (val s = state) {
        is SettingsScreenState.Loading -> CairnPage {}
        is SettingsScreenState.Ready -> SettingsPage(
            s,
            onRateChange = model::editRate,
            onCommitRate = model::commitRate,
            onChoose = model::chooseAppearance,
            onOptimise = model::optimise,
            onCancelOptimise = model::cancelOptimise,
        )
        is SettingsScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("settings-failed")) }
    }
}

/**
 * The page (ADR-0042): the heading, then two groups, each a bold name at body size over a caption,
 * (the egui build's rule between two blocks of a group is drawn when a group holds a second block).
 * Nothing is pinned to the reach line.
 * *This device* holds Appearance alone: no Files block and no sync block (ADR-0051 §3, §5).
 */
@Composable
fun SettingsPage(
    state: SettingsScreenState.Ready,
    onRateChange: (String) -> Unit,
    onCommitRate: () -> Unit,
    onChoose: (ThemeChoice) -> Unit,
    onOptimise: () -> Unit,
    onCancelOptimise: () -> Unit,
) {
    val focus = LocalFocusManager.current
    CairnPage(Modifier.testTag("settings-screen")) {
        Column(
            Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                // A tap elsewhere on the page ends the edit, which commits the rate.
                .pointerInput(Unit) { detectTapGestures { focus.clearFocus() } }
                .padding(top = gap(2), bottom = gap(2)),
        ) {
            CairnText(stringResource(R.string.settings_heading), Modifier.testTag("settings-heading"), tier = Tier.Heading)
            Spacer(Modifier.height(gap(2)))

            GroupHeading(stringResource(R.string.settings_studying), stringResource(R.string.settings_studying_caption), "group-studying")
            RateBlock(state, onRateChange, onCommitRate)
            // The rule between two blocks of a group: three units, a line, two units (as the egui build draws it).
            Spacer(Modifier.height(gap(3)))
            CairnRule()
            Spacer(Modifier.height(gap(2)))
            OptimiseBlock(state, onOptimise, onCancelOptimise)

            Spacer(Modifier.height(gap(2)))
            GroupHeading(stringResource(R.string.settings_device), stringResource(R.string.settings_device_caption), "group-device")
            AppearanceBlock(state, onChoose)
        }
    }
}

/** A group's name in the bold face at body size, and what its settings have in common under it. */
@Composable
private fun GroupHeading(name: String, caption: String, tag: String) {
    CairnText(name, Modifier.testTag(tag), family = Family.Bold)
    CairnFieldLabel(caption, Modifier.testTag("$tag-caption"))
    Spacer(Modifier.height(gap(2)))
}

@Composable
private fun RateBlock(state: SettingsScreenState.Ready, onChange: (String) -> Unit, onCommit: () -> Unit) {
    val focus = LocalFocusManager.current
    var focused by remember { mutableStateOf(false) }
    CairnFieldLabel(stringResource(R.string.settings_rate_label), Modifier.testTag("label-New cards a day"))
    CairnTextField(
        state.rate,
        onChange,
        Modifier
            .testTag("rate-field")
            // Losing focus is a completed edit: it commits, if what is typed parses.
            .onFocusChanged {
                if (focused && !it.isFocused) onCommit()
                focused = it.isFocused
            },
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
        // *Done* closes the keyboard; giving up focus is what commits, so a field not in focus commits here.
        keyboardActions = KeyboardActions(onDone = { if (focused) focus.clearFocus() else onCommit() }),
    )
    if (state.rateRefused) {
        Spacer(Modifier.height(gap(1)))
        CairnText(stringResource(R.string.settings_rate_refused), Modifier.testTag("rate-refused"))
    }
    Spacer(Modifier.height(gap(1)))
    CairnText(stringResource(R.string.settings_rate_consequence), Modifier.testTag("rate-consequence"))
}

@Composable
private fun AppearanceBlock(state: SettingsScreenState.Ready, onChoose: (ThemeChoice) -> Unit) {
    CairnFieldLabel(stringResource(R.string.settings_appearance_label))
    val options = listOf(
        Triple(ThemeChoice.SYSTEM, R.string.settings_appearance_system, "appearance-system"),
        Triple(ThemeChoice.LIGHT, R.string.settings_appearance_light, "appearance-light"),
        Triple(ThemeChoice.DARK, R.string.settings_appearance_dark, "appearance-dark"),
    )
    // The current choice is marked the way the destinations row marks the current place.
    CairnDestinationsRow(
        options.map { (choice, label, tag) ->
            Destination(stringResource(label), state.choice == choice, tag) { onChoose(choice) }
        },
    )
    if (state.appearanceRefused) {
        Spacer(Modifier.height(gap(1)))
        CairnText(stringResource(R.string.settings_appearance_refused), Modifier.testTag("appearance-refused"))
    }
    Spacer(Modifier.height(gap(1)))
    CairnText(stringResource(R.string.settings_appearance_note), Modifier.testTag("appearance-note"))
}

/**
 * Optimise under the field label *Scheduler* (ADR-0014 §2-§4): at rest the full-width button, the nudge
 * and, after a write, the completion sentence; in flight the progress in the button's place and a
 * full-width Cancel. The nudge is a fact; the sentence is said here from the string resources.
 */
@Composable
private fun OptimiseBlock(state: SettingsScreenState.Ready, onOptimise: () -> Unit, onCancel: () -> Unit) {
    CairnFieldLabel(stringResource(R.string.settings_scheduler_label), Modifier.testTag("label-Scheduler"))
    when (val run = state.run) {
        is OptimiseRun.Idle -> {
            CairnControl(stringResource(R.string.settings_optimise), onOptimise, Modifier.fillMaxWidth().testTag("optimise-button"))
            Spacer(Modifier.height(gap(1)))
            state.nudge?.let { CairnText(nudgeSentence(it), Modifier.testTag("nudge-text")) }
            if (state.updated) {
                Spacer(Modifier.height(gap(1)))
                CairnText(stringResource(R.string.settings_optimise_done), Modifier.testTag("optimise-done"))
            }
        }
        is OptimiseRun.Preparing -> {
            CairnText(stringResource(R.string.settings_optimise_preparing), Modifier.testTag("optimise-preparing"))
            Spacer(Modifier.height(gap(1)))
            CairnProgress(null, Modifier.testTag("optimise-indicator"))
            Spacer(Modifier.height(gap(1)))
            CairnControl(stringResource(R.string.settings_optimise_cancel), onCancel, Modifier.fillMaxWidth().testTag("optimise-cancel"))
        }
        is OptimiseRun.Training -> {
            val fraction = if (run.total == 0L) 0f else run.current.toFloat() / run.total.toFloat()
            CairnProgress(fraction, Modifier.testTag("optimise-progress"))
            Spacer(Modifier.height(gap(1)))
            CairnControl(stringResource(R.string.settings_optimise_cancel), onCancel, Modifier.fillMaxWidth().testTag("optimise-cancel"))
        }
    }
}

/** The nudge in words (ADR-0014 §2): the counts grouped by thousands and the nouns agreeing with them. */
@Composable
private fun nudgeSentence(nudge: Nudge): String = when (nudge) {
    is Nudge.Standard -> stringResource(R.string.nudge_standard, counted(R.plurals.nudge_times, nudge.reviewsTotal))
    is Nudge.Fitted -> stringResource(
        R.string.nudge_fitted,
        counted(R.plurals.nudge_reviews, nudge.fittedOver),
        counted(R.plurals.nudge_times, nudge.reviewsSince),
    )
}

@Composable
private fun counted(plural: Int, n: Long): String =
    pluralStringResource(plural, n.coerceAtMost(Int.MAX_VALUE.toLong()).toInt(), String.format(java.util.Locale.ENGLISH, "%,d", n))
