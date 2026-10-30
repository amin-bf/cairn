package dev.cairn.app

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.cairn.design.Cairn
import dev.cairn.design.CairnText

class MainActivity : CairnActivity() {
    @Composable
    override fun Screen() = Placeholder()
}

/** The walking skeleton's one screen: how many cards are due, read through the binding. */
@Composable
private fun Placeholder() {
    val owner = (LocalContext.current.applicationContext as CairnApplication).collection
    val model = viewModel { DueViewModel(owner) }
    val state by model.state.collectAsStateWithLifecycle()
    Box(Modifier.fillMaxSize().background(Cairn.colors.page), contentAlignment = Alignment.Center) {
        when (val s = state) {
            is DueState.Loading -> Unit
            is DueState.Known ->
                CairnText(
                    pluralStringResource(R.plurals.due_count, s.due, s.due),
                    Modifier.testTag("due-count"),
                )
            is DueState.Failed -> CairnText(stringResource(R.string.due_unavailable), Modifier.testTag("due-failed"))
        }
    }
}
