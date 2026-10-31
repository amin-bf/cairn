package dev.cairn.app

import android.app.Activity
import android.content.Context
import android.net.Uri
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivityResultRegistryOwner
import androidx.activity.compose.setContent
import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.ActivityResultRegistryOwner
import androidx.activity.result.contract.ActivityResultContract
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.core.app.ActivityOptionsCompat
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.style.ResolvedTextDirection
import androidx.compose.ui.unit.dp
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.FileArrival
import dev.cairn.ffi.FilePreview
import dev.cairn.ffi.ImportGate
import dev.cairn.ffi.ThemeChoice
import dev.cairn.ffi.benchFiles
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.FileOutputStream
import java.util.concurrent.atomic.AtomicReference

/**
 * *Open a file…* reaches the import preview (#242, ADR-0052, ADR-0022 §3, ADR-0041 §1-§3): every
 * state is reached from a real bench file over `Fixture::Decks`, through the real binding, with the
 * picker's result stubbed through a test `ActivityResultRegistry`.
 */
@RunWith(AndroidJUnit4::class)
class ImportPreviewTest {
    @get:Rule
    val rule = createAndroidComposeRule<ComponentActivity>()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "import-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    // The bench's files in the order the set builds them.
    private val bench = benchFiles(context.cacheDir.path)
    private val update = bench[0]
    private val several = bench[1]
    private val archive = bench[2]
    private val unreadable = bench[3]

    /** What the stubbed picker answers with; null is a picker closed with nothing chosen. */
    private val chosen = AtomicReference<Uri?>(null)

    /** What the app asked the picker for, as the picker was launched. */
    private val asked = AtomicReference<Any?>(null)

    private val picker = object : ActivityResultRegistryOwner {
        override val activityResultRegistry = object : ActivityResultRegistry() {
            override fun <I, O> onLaunch(requestCode: Int, contract: ActivityResultContract<I, O>, input: I, options: ActivityOptionsCompat?) {
                asked.set(input)
                val uri = chosen.get()
                if (uri == null) {
                    dispatchResult(requestCode, Activity.RESULT_CANCELED, null)
                } else {
                    dispatchResult(requestCode, Activity.RESULT_OK, android.content.Intent().setData(uri))
                }
            }
        }
    }

    @Before
    fun serve() {
        for (file in bench) BenchProvider.files[file.name] = file.bytes
        runBlocking { owner.use { it.installFixture("decks", System.currentTimeMillis()) } }
    }

    @After
    fun tearDown() {
        BenchProvider.files.clear()
        root.deleteRecursively()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun top(tag: String): Float = rule.onNodeWithTag(tag).fetchSemanticsNode().boundsInRoot.top

    private fun bottom(tag: String): Float = rule.onNodeWithTag(tag).fetchSemanticsNode().boundsInRoot.bottom

    private fun showApp(initial: ThemeChoice = ThemeChoice.LIGHT) {
        val appearance = AppearanceState(initial)
        rule.setContent {
            CompositionLocalProvider(LocalActivityResultRegistryOwner provides picker) {
                CairnAppearance(appearance) { CairnNavHost(owner, keyboardUp = false) }
            }
        }
        waitFor("destinations")
    }

    private fun openFilePicking(file: Uri?) {
        chosen.set(file)
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("open-file")
        rule.onNodeWithTag("open-file").performClick()
    }

    private fun pick(name: String, query: String? = null) {
        openFilePicking(BenchProvider.uri(name, query))
        waitFor("import-screen")
    }

    private fun pressBack() {
        InstrumentationRegistry.getInstrumentation().runOnMainSync { rule.activity.onBackPressedDispatcher.onBackPressed() }
        rule.waitForIdle()
    }

    private fun rawLines(): List<String> =
        rule.onAllNodes(hasTestTag("import-line")).fetchSemanticsNodes().map { it.config[SemanticsProperties.Text].joinToString("") { t -> t.text } }

    /** The lines without the direction marks `unicodeWrap` puts round a name: what a person reads. */
    private fun lines(): List<String> = rawLines().map { line -> line.filterNot { it in '\u200E'..'\u200F' || it in '\u202A'..'\u202E' || it in '\u2066'..'\u2069' } }

    private fun direction(tag: String): ResolvedTextDirection {
        val results = mutableListOf<TextLayoutResult>()
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsActions.GetTextLayoutResult].action?.invoke(results)
        return results.single().getParagraphDirection(0)
    }

    // --- The Files block (ADR-0051 §5).

    @Test
    fun the_files_block_sits_between_the_group_heading_and_appearance() {
        showApp()
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("open-file")
        assertEquals("Files", text("label-Files"))
        assertEquals("Choose a file to see what importing or restoring it would do.", text("files-line"))
        assertEquals("Open a file…", text("open-file"))
        assertTrue(top("group-device") < top("label-Files"))
        assertTrue(top("label-Files") < top("open-file"))
        // The line sits under the button, as the Optimise block's does.
        assertTrue(top("open-file") < top("files-line"))
        // Appearance is below the fold since the backup row came in, where the clipped bounds can't place it.
        val position = { tag: String -> rule.onNodeWithTag(tag).fetchSemanticsNode().positionInRoot.y }
        assertTrue(position("files-line") < position("label-Appearance"))
    }

    @Test
    fun a_picker_closed_with_nothing_does_nothing() {
        showApp()
        openFilePicking(null)
        rule.waitForIdle()
        assertFalse(exists("import-screen"))
        assertTrue(exists("open-file"))
        assertTrue(exists("destinations"))
    }

    @Test
    fun the_picker_is_asked_for_a_file_of_any_type() {
        showApp()
        openFilePicking(null)
        rule.waitForIdle()
        // A deck file has no type a provider is sure to know, so nothing is filtered out (ADR-0047 §1).
        assertEquals(listOf("*/*"), (asked.get() as Array<*>).toList())
    }

    @Test
    fun after_a_killed_process_the_import_route_comes_back_on_review() {
        // The models live in a store the test can throw away, as a killed process does; the saved
        // back stack outlives it.
        var store = ViewModelStore()
        val stores = object : ViewModelStoreOwner {
            override val viewModelStore: ViewModelStore get() = store
        }
        val restoration = StateRestorationTester(rule)
        val appearance = AppearanceState(ThemeChoice.LIGHT)
        restoration.setContent {
            CompositionLocalProvider(LocalActivityResultRegistryOwner provides picker, LocalViewModelStoreOwner provides stores) {
                CairnAppearance(appearance) { CairnNavHost(owner, keyboardUp = false) }
            }
        }
        waitFor("destinations")
        pick(update.name)
        store.clear()
        store = ViewModelStore()
        restoration.emulateSavedInstanceStateRestore()
        // The bytes were never saved, so the preview is not restored and the app opens on Review (#241).
        waitFor("destinations")
        assertFalse(exists("import-screen"))
        assertEquals(true, rule.onNodeWithTag("destination-review").fetchSemanticsNode().config[SemanticsProperties.Selected])
    }

    // --- The preview and its gate (ADR-0022 §3, ADR-0041 §1-§3).

    @Test
    fun the_update_deck_states_its_plan_under_the_header_and_offers_import_and_cancel() {
        showApp()
        pick(update.name)
        assertEquals(update.name, text("import-heading"))
        assertEquals("by Marjan Rahimi · CC BY-SA 4.0", text("import-claim-0"))
        assertEquals("A1 vocabulary for the first ten chapters.", text("import-claim-1"))
        assertTrue(exists("import-header-rule"))
        // The rule has gap(2) on either side (ADR-0041 §1).
        val gap2 = with(rule.density) { dev.cairn.design.gap(2).toPx() }
        assertEquals(gap2, top("import-header-rule") - bottom("import-claim-1"), 1f)
        assertEquals(gap2, top("import-deck-0") - bottom("import-header-rule"), 1f)
        assertEquals("French A1", text("import-deck-0"))
        assertEquals("updating a deck you already have", text("import-caption-0"))
        assertEquals(
            listOf(
                "6 new notes, 7 already yours",
                "1 note moving in from Unfiled",
                "4 notes moving in from Vocabulaire de la Révolution française",
                "3 of your notes will be deleted",
                "Renaming your Français to French A1",
                "Vocabulaire de la Révolution française will be left empty",
            ),
            lines().take(6),
        )
        // A name inside a sentence is wrapped, so its direction cannot reorder the application's words.
        val held = android.text.BidiFormatter.getInstance().unicodeWrap("Vocabulaire de la Révolution française")
        assertTrue(rawLines()[5].startsWith(held))
        assertEquals("Your review history is untouched.", lines().last())
        assertTrue(exists("gate-import") && exists("gate-cancel"))
        assertFalse(exists("gate-close"))
        // The destinations row is hidden while the preview has the whole screen.
        assertFalse(exists("destinations"))
    }

    @Test
    fun the_three_deck_file_sets_the_persian_name_in_its_own_direction_and_draws_no_header() {
        showApp()
        pick(several.name)
        assertFalse(exists("import-claim-0"))
        assertFalse(exists("import-header-rule"))
        assertEquals("فارسی", text("import-deck-0"))
        assertEquals("Deutsch", text("import-deck-1"))
        assertEquals("Nederlands", text("import-deck-2"))
        assertEquals(ResolvedTextDirection.Rtl, direction("import-deck-0"))
        assertEquals(ResolvedTextDirection.Rtl, direction("import-caption-0"))
        assertEquals(ResolvedTextDirection.Ltr, direction("import-deck-1"))
        assertEquals("new deck", text("import-caption-1"))
        assertTrue(exists("gate-import") && exists("gate-cancel"))
    }

    @Test
    fun another_collections_archive_is_refused_in_one_sentence_with_a_lone_close() {
        // The bench archive is a backup of `Fixture::Decks` under an id no installed fixture holds,
        // and this collection has notes of its own: the identity gate refuses it (ADR-0016 §10).
        showApp()
        pick(archive.name)
        assertEquals(archive.name, text("import-heading"))
        assertEquals(
            "This archive is from a different collection. To use it on this device: back up what is here, clear the app's data, restore.",
            text("import-refusal"),
        )
        assertTrue(exists("gate-close"))
        assertFalse(exists("gate-import") || exists("gate-cancel"))
        assertFalse(exists("import-line"))
    }

    @Test
    fun a_file_that_cannot_be_parsed_is_unreadable() {
        showApp()
        pick(unreadable.name)
        assertEquals("This file could not be read as a deck.", text("import-refusal"))
        assertTrue(exists("gate-close"))
    }

    @Test
    fun a_read_the_provider_fails_is_unreadable() {
        showApp()
        pick(update.name, query = "fails")
        assertEquals("This file could not be read as a deck.", text("import-refusal"))
        assertTrue(exists("gate-close"))
    }

    @Test
    fun a_share_with_no_name_is_headed_a_deck_file() {
        showApp()
        pick(update.name, query = "nameless")
        assertEquals("A deck file", text("import-heading"))
    }

    @Test
    fun a_file_that_changes_nothing_offers_a_lone_close() {
        runBlocking {
            owner.use {
                it.openFile(FileArrival.PICKED, update.name, update.bytes, true).apply()
            }
        }
        showApp()
        pick(update.name)
        assertEquals("a deck you already have", text("import-caption-0"))
        assertEquals(listOf("Nothing will change.", "Your review history is untouched."), lines())
        assertTrue(exists("gate-close"))
        assertFalse(exists("gate-import") || exists("gate-cancel"))
    }

    @Test
    fun the_kinds_a_file_adopts_are_each_wrapped_in_their_line() {
        // No bench file adopts a kind, so the plan is stated directly: the names are the file's own
        // and so hostile, like every other name in a sentence (ADR-0041 §2).
        val plan = dev.cairn.ffi.ImportPlanFacts(
            author = "",
            licence = "",
            description = "",
            decks = listOf(
                dev.cairn.ffi.DeckEffects(
                    name = "French A1",
                    path = dev.cairn.ffi.DeckPath.CREATE,
                    newNotes = 1u,
                    alreadyYours = 0u,
                    movingIn = emptyList(),
                    deleted = 0u,
                    renamedFrom = null,
                    noChange = false,
                    revisionConflict = false,
                ),
            ),
            adoptedKinds = listOf("واژه", "Cloze"),
            emptiedDecks = emptyList(),
            gate = ImportGate.DECIDE,
        )
        rule.setContent { CairnTheme(false) { ImportPage("kinds.cdeck", FilePreview.Plan(plan), onImport = {}, onLeave = {}) } }
        val wrap = android.text.BidiFormatter.getInstance()
        val expected = "Adds a card type this build doesn't have: ${wrap.unicodeWrap("واژه")}, ${wrap.unicodeWrap("Cloze")}"
        assertTrue(rawLines().contains(expected))
    }

    // --- Leaving (ADR-0052: Back is the gate's dismissal).

    @Test
    fun cancel_close_and_back_return_to_settings() {
        showApp()
        pick(update.name)
        rule.onNodeWithTag("gate-cancel").performClick()
        waitFor("open-file")
        assertFalse(exists("import-screen"))
        assertTrue(exists("destinations"))

        chosen.set(BenchProvider.uri(archive.name))
        rule.onNodeWithTag("open-file").performClick()
        waitFor("gate-close")
        rule.onNodeWithTag("gate-close").performClick()
        waitFor("open-file")
        assertFalse(exists("import-screen"))

        chosen.set(BenchProvider.uri(several.name))
        rule.onNodeWithTag("open-file").performClick()
        waitFor("import-screen")
        pressBack()
        waitFor("open-file")
        assertFalse(exists("import-screen"))
    }

    // --- Screenshots, in both themes.

    private var scene by androidx.compose.runtime.mutableStateOf<@androidx.compose.runtime.Composable () -> Unit>({})
    private var darkScene by androidx.compose.runtime.mutableStateOf(false)

    private fun shoot(name: String, dark: Boolean, content: @androidx.compose.runtime.Composable () -> Unit) {
        val dir = context.getExternalFilesDir("import-shots")!!
        darkScene = dark
        scene = content
        rule.waitForIdle()
        val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
        FileOutputStream(File(dir, "$name-${if (dark) "dark" else "light"}.png")).use {
            bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
        assertEquals(pageArgb(dark), bitmap.getPixel(2, bitmap.height / 2))
    }

    @Test
    fun a_file_replacing_the_preview_opens_at_the_top_of_its_page() {
        // A short host, so the bench's longest preview scrolls; the files arrive as another app hands
        // them over (#244). The same file twice: the second is a new arrival, though its name is the same.
        val incoming = IncomingFiles(context as android.app.Application)
        rule.setContent {
            Box(Modifier.height(240.dp)) {
                CairnAppearance(AppearanceState(ThemeChoice.LIGHT)) {
                    CairnNavHost(owner, keyboardUp = false, incoming = incoming)
                }
            }
        }
        waitFor("destinations")
        fun heading() = rule.onNodeWithTag("import-heading").fetchSemanticsNode().positionInRoot.y
        incoming.arrive(ArrivingFile(FileArrival.OPENED, BenchProvider.uri(several.name)))
        waitFor("import-deck-0")
        val atTop = heading()
        rule.onNode(hasScrollAction()).performSemanticsAction(SemanticsActions.ScrollBy) { it(0f, 10_000f) }
        rule.waitForIdle()
        assertTrue("the page scrolled: ${heading()} against $atTop", heading() < atTop)
        incoming.arrive(ArrivingFile(FileArrival.OPENED, BenchProvider.uri(several.name)))
        rule.waitUntil(10_000) { incoming.pending.value == null }
        rule.waitForIdle()
        assertEquals(atTop, heading(), 1f)
    }

    private fun previewOf(file: dev.cairn.ffi.BenchFileFact) = runBlocking {
        owner.use { it.openFile(FileArrival.PICKED, file.name, file.bytes, true).preview() }
    }

    @Test
    fun the_files_block_and_the_previews_are_drawn_in_both_themes() {
        rule.setContent { CairnTheme(darkScene) { scene() } }
        for (dark in listOf(false, true)) {
            shoot("files-block", dark) {
                SettingsPage(SettingsScreenState.Ready("5", ThemeChoice.LIGHT), {}, {}, {}, {}, {})
            }
            assertTrue(exists("open-file"))
            for ((name, file) in listOf("plan-header" to update, "persian" to several, "refusal" to archive)) {
                val preview = previewOf(file)
                shoot(name, dark) { ImportPage(file.name, preview, onImport = {}, onLeave = {}) }
                assertTrue(exists("import-screen"))
            }
        }
    }
}
