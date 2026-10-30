package dev.cairn.app

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.createSavedStateHandle
import androidx.navigation.NavType
import androidx.navigation.navArgument
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

/** How often Settings polls an Optimise run while the screen is started (client-stack rule 10): short, so the bar moves. */
const val SETTINGS_TICK_MS = 250L

private const val REVIEW = "review"
private const val NOTES = "notes"
private const val SETTINGS = "settings"
private const val LEECHES = "leeches"
private const val EDITOR = "editor?$ARG_NOTE={$ARG_NOTE}&$ARG_KIND={$ARG_KIND}&$ARG_DECK={$ARG_DECK}"

/** The editor's route: a stored [note], or a draft of [kind] to be filed to [deck]. */
internal fun editorRoute(note: String? = null, kind: String? = null, deck: String? = null): String =
    buildString {
        append("editor?")
        listOf(ARG_NOTE to note, ARG_KIND to kind, ARG_DECK to deck)
            .filter { it.second != null }
            .joinTo(this, "&") { "${it.first}=${Uri.encode(it.second)}" }
    }

/**
 * The app's places on the Navigation back stack (ADR-0048 §1, §6): Review, where it starts, Notes and
 * Settings, with the destinations row above them. Moving between them keeps each one's state, and the
 * system Back from Notes or Settings returns to Review.
 */
@Composable
fun CairnNavHost(
    owner: CollectionAccess,
    keyboardUp: Boolean = rememberKeyboardUp(),
    settingsTickMs: Long = SETTINGS_TICK_MS,
) {
    val nav = rememberNavController()
    val route = nav.currentBackStackEntryAsState().value?.destination?.route
    val destinations = listOf(
        Destination(stringResource(R.string.destination_review), route == REVIEW || route == LEECHES, "destination-review") { nav.goTo(REVIEW) },
        Destination(stringResource(R.string.destination_notes), route == NOTES, "destination-notes") { nav.goTo(NOTES) },
        Destination(stringResource(R.string.destination_settings), route == SETTINGS, "destination-settings") { nav.goTo(SETTINGS) },
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
                // Coming back from the editor (or from Notes, or the background) the shown card is
                // re-read: the edit shows on it, or the sitting moves on (ADR-0048 §6). Asking is idle.
                LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { model.refresh() }
                ReviewScreen(
                    model,
                    onEditNote = { nav.navigate(editorRoute(note = it)) },
                    onLeeches = { nav.navigate(LEECHES) },
                )
            }
            composable(LEECHES) {
                val model = viewModel { LeechViewModel(owner) }
                // Coming back from the editor the list is read again: the edit may have changed what is a leech.
                LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { model.refresh() }
                LeechScreen(
                    model,
                    onEdit = { nav.navigate(editorRoute(note = it)) },
                    onBack = { nav.popBackStack() },
                )
            }
            composable(NOTES) {
                NotesScreen(
                    viewModel { NotesViewModel(owner) },
                    onOpen = { nav.navigate(editorRoute(note = it)) },
                    onCreate = { deck -> nav.navigate(editorRoute(kind = NEW_KIND, deck = deck)) },
                )
            }
            composable(SETTINGS) {
                val appearance = LocalAppearance.current
                val model = viewModel { SettingsViewModel(owner, appearance) }
                // Coming back to the page it is read again; a field mid-edit keeps what was typed.
                LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { model.refresh() }
                // A run is polled while the screen is started, and not while it is stopped or left (client-stack rule 10).
                val lifecycle = LocalLifecycleOwner.current.lifecycle
                LaunchedEffect(model, settingsTickMs) {
                    lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
                        while (true) {
                            delay(settingsTickMs)
                            model.tick()
                        }
                    }
                }
                SettingsScreen(model)
            }
            composable(
                EDITOR,
                arguments = listOf(ARG_NOTE, ARG_KIND, ARG_DECK).map {
                    navArgument(it) {
                        type = NavType.StringType
                        nullable = true
                        defaultValue = null
                    }
                },
            ) {
                val model = viewModel { EditorViewModel(owner, createSavedStateHandle()) }
                EditorScreen(
                    model,
                    onDone = { nav.popBackStack() },
                    // The fresh draft takes this one's place, so Back from it returns to where this was opened from.
                    onNewNote = { kind, deck ->
                        nav.navigate(editorRoute(kind = kind, deck = deck)) { popUpTo(EDITOR) { inclusive = true } }
                    },
                )
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
