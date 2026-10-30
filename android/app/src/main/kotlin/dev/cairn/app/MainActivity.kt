package dev.cairn.app

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText

class MainActivity : CairnActivity() {
    @Composable
    override fun Screen() {
        val owner = (LocalContext.current.applicationContext as CairnApplication).collection
        ReviewScreen(viewModel { ReviewViewModel(owner) })
    }
}

/** Review: the entrance until a sitting is begun. */
@Composable
fun ReviewScreen(model: ReviewViewModel) {
    val state by model.state.collectAsStateWithLifecycle()
    when (val s = state) {
        is ReviewScreenState.Loading -> CairnPage {}
        is ReviewScreenState.Entrance -> ReviewEntrance(s.facts, onStart = model::start)
        // Until the sitting ticket lands, beginning one shows the count it was begun with.
        is ReviewScreenState.Sitting ->
            CairnPage { CairnText(stringResource(R.string.review_sitting_stub, s.chosen), Modifier.testTag("sitting-stub")) }
        is ReviewScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("review-failed")) }
    }
}
