package dev.cairn.app

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText

class MainActivity : CairnActivity() {
    @Composable
    override fun Screen() {
        val owner = (LocalContext.current.applicationContext as CairnApplication).collection
        CairnNavHost(owner)
    }
}

/** Review: the entrance until a sitting is begun, then the sitting until it ends. */
@Composable
fun ReviewScreen(model: ReviewViewModel, onEditNote: (noteId: String) -> Unit = {}, onLeeches: () -> Unit = {}) {
    val state by model.state.collectAsStateWithLifecycle()
    when (val s = state) {
        is ReviewScreenState.Loading -> CairnPage {}
        is ReviewScreenState.Entrance -> ReviewEntrance(s.facts, onStart = model::start, onLeeches = onLeeches)
        is ReviewScreenState.Sitting -> ReviewSitting(
            chosen = s.chosen,
            graded = s.graded,
            card = s.card,
            checkpoint = s.checkpoint,
            onReveal = model::reveal,
            onGrade = model::grade,
            onKeepGoing = model::keepGoing,
            onFinish = model::finishHere,
            onEditNote = { onEditNote(s.card.noteId) },
        )
        is ReviewScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("review-failed")) }
    }
}
