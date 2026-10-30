package dev.cairn.app

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.cairn.design.CairnConfirmRow
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnRule
import dev.cairn.design.CairnText
import dev.cairn.design.Family
import dev.cairn.design.gap

/**
 * The Development block (ADR-0051 §1, §6): reset and the fixture bench, after a rule at the end of
 * Settings under its own bold heading. Each control asks in its own row before it replaces the
 * collection. Only a build made with `CAIRN_DEVELOPMENT=1` has this source set.
 */
@Composable
fun DevelopmentBlock(access: CollectionAccess, onRestart: () -> Unit) {
    val model = viewModel { DevelopmentViewModel(access) }
    val asking by model.asking.collectAsStateWithLifecycle()
    Column {
        Spacer(Modifier.height(gap(2)))
        CairnRule()
        Spacer(Modifier.height(gap(2)))
        CairnText(stringResource(R.string.development_heading), Modifier.testTag("development-heading"), family = Family.Bold)
        Spacer(Modifier.height(gap(2)))

        CairnText(stringResource(R.string.development_reset_sentence), Modifier.testTag("reset-sentence"))
        Spacer(Modifier.height(gap(1)))
        if (asking == Asking.Reset) {
            CairnConfirmRow(
                question = stringResource(R.string.development_reset_question),
                confirm = stringResource(R.string.development_reset_confirm),
                keep = stringResource(R.string.development_keep),
                onConfirm = { model.reset(onRestart) },
                onKeep = model::keep,
                questionModifier = Modifier.testTag("reset-question"),
                confirmModifier = Modifier.testTag("reset-confirm"),
                keepModifier = Modifier.testTag("reset-keep"),
            )
        } else {
            CairnControl(
                stringResource(R.string.development_reset_control),
                onClick = { model.ask(Asking.Reset) },
                modifier = Modifier.testTag("reset-control"),
            )
        }

        Spacer(Modifier.height(gap(2)))
        CairnText(stringResource(R.string.development_bench_sentence), Modifier.testTag("bench-sentence"))
        Spacer(Modifier.height(gap(1)))
        for (fixture in model.fixtures) {
            if (asking == Asking.Install(fixture.key)) {
                CairnConfirmRow(
                    question = stringResource(R.string.development_install_question, fixture.label),
                    confirm = stringResource(R.string.development_install_confirm),
                    keep = stringResource(R.string.development_keep),
                    onConfirm = { model.install(fixture.key, onRestart) },
                    onKeep = model::keep,
                    questionModifier = Modifier.testTag("fixture-question"),
                    confirmModifier = Modifier.testTag("fixture-confirm"),
                    keepModifier = Modifier.testTag("fixture-keep"),
                )
            } else {
                CairnControl(
                    fixture.label,
                    onClick = { model.ask(Asking.Install(fixture.key)) },
                    modifier = Modifier.testTag("fixture-${fixture.key}"),
                )
            }
            Spacer(Modifier.height(gap(1)))
        }
        DevelopmentBench.said?.let {
            CairnText(it, Modifier.testTag("bench-result"))
        }
    }
}
