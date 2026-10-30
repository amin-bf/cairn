package dev.cairn.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { Placeholder() }
    }
}

/** The walking skeleton's one screen: how many cards are due, read through the binding. */
@androidx.compose.runtime.Composable
private fun Placeholder() {
    val owner = (androidx.compose.ui.platform.LocalContext.current.applicationContext as CairnApplication).collection
    val model = viewModel { DueViewModel(owner) }
    val state by model.state.collectAsStateWithLifecycle()
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        when (val s = state) {
            is DueState.Loading -> Unit
            is DueState.Known ->
                Text(
                    pluralStringResource(R.plurals.due_count, s.due, s.due),
                    Modifier.testTag("due-count"),
                )
            is DueState.Failed -> Text(stringResource(R.string.due_unavailable), Modifier.testTag("due-failed"))
        }
    }
}
