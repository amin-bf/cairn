package dev.cairn.app

import android.content.Intent
import android.os.Bundle
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.runtime.getValue
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText

class MainActivity : CairnActivity() {
    private val incoming by lazy { ViewModelProvider(this)[IncomingFiles::class.java] }
    private val handledArrivals by lazy { HandledArrivals.of(this) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // An intent is handled once (#241): a recreation brings the one already handled back, and so
        // does a return from the recents screen, which marks it as launched from history.
        val fromHistory = intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY != 0
        if (savedInstanceState == null && !fromHistory) startedBy(intent)
    }

    // singleTask: a file opened from another app while this one runs comes here (ADR-0052 §2). It is
    // handled here and not kept as the activity's intent, which nothing reads again. It is the person's
    // new choice, never the system's replay, so it is not weighed against the record.
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        arrivingFrom(intent)?.let(incoming::arrive)
    }

    // The system replays a task's root intent after an upgrade (#255), and that start is the only one it
    // replays into: the intent that starts the app is acted on once.
    private fun startedBy(intent: Intent) {
        val file = arrivingFrom(intent) ?: return
        if (handledArrivals.isReplay(file)) return
        handledArrivals.handled(file)
        incoming.arrive(file)
    }

    @Composable
    override fun Screen() {
        val owner = (LocalContext.current.applicationContext as CairnApplication).collection
        CairnNavHost(owner, incoming = incoming)
    }
}

/**
 * Review: the entrance until a sitting is begun, then the sitting until it ends — and, when cards crossed
 * the leech floor in it, the pointer in the entrance's place until it is answered.
 */
@Composable
fun ReviewScreen(model: ReviewViewModel, onEditNote: (noteId: String) -> Unit = {}, onLeeches: () -> Unit = {}) {
    val state by model.state.collectAsStateWithLifecycle()
    when (val s = state) {
        is ReviewScreenState.Loading -> CairnPage {}
        is ReviewScreenState.Entrance -> ReviewEntrance(s.facts, onStart = model::start, onLeeches = onLeeches)
        is ReviewScreenState.Pointer -> LeechPointer(
            s.crossed,
            onShow = {
                model.answerPointer()
                onLeeches()
            },
            onDismiss = model::answerPointer,
        )
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
