package dev.cairn.app

import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.navigation.NavController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.ffi.FileArrival
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.cairn.design.CairnShell
import dev.cairn.design.Destination
import dev.cairn.design.rememberKeyboardUp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** How often a sitting is asked again while nothing is touched, so the check-in can fall due. */
private const val TICK_MS = 15_000L

/** How often Settings polls an Optimise run while the screen is started (client-stack rule 10): short, so the bar moves. */
const val SETTINGS_TICK_MS = 250L

private const val REVIEW = "review"
private const val NOTES = "notes"
private const val SETTINGS = "settings"
private const val LEECHES = "leeches"
private const val IMPORT = "import"
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
    incoming: IncomingFiles? = null,
) {
    // After a reset or a fixture install the app starts afresh (#237): the whole graph is composed again
    // under a new key, so the back stack is Review and no place keeps a ViewModel of the replaced
    // collection, and the appearance is read again from the new one.
    var generation by remember { mutableIntStateOf(0) }
    val appearance = LocalAppearance.current
    val scope = rememberCoroutineScope()
    key(generation) {
        CairnPlaces(owner, generation, keyboardUp, settingsTickMs, incoming) {
            scope.launch {
                appearance.choice = owner.themeChoiceOrSystem()
                generation++
            }
        }
    }
}

@Composable
private fun CairnPlaces(
    owner: CollectionAccess,
    generation: Int,
    keyboardUp: Boolean,
    settingsTickMs: Long,
    incoming: IncomingFiles?,
    onRestart: () -> Unit,
) {
    val nav = rememberNavController()
    // The file arrives before the screen that shows it, so the model is the activity's: it survives a
    // rotation, and a replaced collection (#237) gets a fresh one.
    val importModel = viewModel(key = "import-$generation") { ImportViewModel(owner) }
    // The editor on screen, if one is: a file arriving settles it first (ADR-0021 §7, ADR-0052 §2).
    var openEditor by remember { mutableStateOf<EditorViewModel?>(null) }
    val resolver = LocalContext.current.contentResolver
    // Every way in reaches the one preview (ADR-0022 §6): the document is read once, and the preview
    // takes the screen over the place the person was on. A preview already up is replaced (#241).
    val arrive: suspend (FileArrival, Uri) -> Unit = { arrival, uri ->
        importModel.open(arrival, readDocument(resolver, uri))
        // The back stack is the main thread's. The scope is too, but under an unconfined dispatcher
        // (the compose test's) the read resumes on the IO thread.
        withContext(Dispatchers.Main) { nav.navigate(IMPORT) { launchSingleTop = true } }
    }
    if (incoming != null) {
        LaunchedEffect(incoming) {
            // A newer file cancels the one still being read: the newest is the one the person chose.
            incoming.pending.filterNotNull().collectLatest { file ->
                openEditor?.settled()
                arrive(file.arrival, file.uri)
                incoming.handled(file)
            }
        }
    }
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
                // The system picker, for any type; a picker closed with nothing chosen does nothing. The
                // document is read once and its grant is never kept (ADR-0047 §1, ADR-0052).
                val scope = rememberCoroutineScope()
                val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
                    if (uri != null) scope.launch { arrive(FileArrival.PICKED, uri) }
                }
                SettingsScreen(
                    model,
                    onOpenFile = { picker.launch(arrayOf("*/*")) },
                    development = { DevelopmentBlock(owner, onRestart) },
                )
            }
            composable(IMPORT) {
                // Cancel, Close and Back are one dismissal: nothing written, nothing kept (ADR-0052).
                val leave = {
                    importModel.clear()
                    nav.popBackStack()
                    Unit
                }
                BackHandler(onBack = leave)
                // The plan is derived on every read, so it is read again on return (ADR-0022 §5).
                LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { importModel.refresh() }
                // After a killed process the route comes back with no file: the preview is not
                // restored, and the app opens on Review (#241). Leaving clears the model too, but
                // has already popped the route by then.
                val held by importModel.state.collectAsStateWithLifecycle()
                if (held is ImportScreenState.Empty) {
                    LaunchedEffect(Unit) {
                        if (nav.currentBackStackEntry?.destination?.route == IMPORT) {
                            nav.navigate(REVIEW) { popUpTo(nav.graph.id) { inclusive = true } }
                        }
                    }
                }
                ImportScreen(importModel, onLeave = leave)
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
                DisposableEffect(model) {
                    openEditor = model
                    onDispose { if (openEditor === model) openEditor = null }
                }
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
    // The import preview takes the whole screen, with no destinations row (ADR-0052).
    CairnShell(destinations, keyboardUp = keyboardUp, showDestinations = route != IMPORT, content = shell)
}

/** Switch destination the way a top-level row does: one entry per place, state kept, no pile-up. */
private fun NavController.goTo(route: String) {
    navigate(route) {
        popUpTo(graph.startDestinationId) { saveState = true }
        launchSingleTop = true
        restoreState = true
    }
}
