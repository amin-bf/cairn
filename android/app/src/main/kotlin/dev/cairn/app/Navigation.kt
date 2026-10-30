package dev.cairn.app

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.navigation.NavController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.cairn.design.CairnShell
import dev.cairn.design.Destination
import dev.cairn.design.rememberKeyboardUp
import kotlinx.coroutines.delay

/** How often a sitting is asked again while nothing is touched, so the check-in can fall due. */
private const val TICK_MS = 15_000L

private const val REVIEW = "review"
private const val NOTES = "notes"

/**
 * The app's places on the Navigation back stack (ADR-0048 §1, §6): Review, where it starts, and Notes,
 * with the destinations row above them. Moving between the two keeps each one's state, and the system
 * Back from Notes returns to Review.
 */
@Composable
fun CairnNavHost(owner: CollectionOwner, keyboardUp: Boolean = rememberKeyboardUp()) {
    val nav = rememberNavController()
    val route = nav.currentBackStackEntryAsState().value?.destination?.route
    val destinations = listOf(
        Destination(stringResource(R.string.destination_review), route == REVIEW, "destination-review") { nav.goTo(REVIEW) },
        Destination(stringResource(R.string.destination_notes), route == NOTES, "destination-notes") { nav.goTo(NOTES) },
    )
    val shell: @Composable () -> Unit = {
        NavHost(nav, startDestination = REVIEW) {
            composable(REVIEW) {
                val model = viewModel { ReviewViewModel(owner) }
                LaunchedEffect(model) {
                    while (true) {
                        delay(TICK_MS)
                        model.tick()
                    }
                }
                ReviewScreen(model)
            }
            composable(NOTES) {
                NotesScreen(viewModel { NotesViewModel(owner) })
            }
        }
    }
    CairnShell(destinations, keyboardUp = keyboardUp, content = shell)
}

/** Switch destination the way a top-level row does: one entry per place, state kept, no pile-up. */
private fun NavController.goTo(route: String) {
    navigate(route) {
        popUpTo(graph.startDestinationId) { saveState = true }
        launchSingleTop = true
        restoreState = true
    }
}
