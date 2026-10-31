package dev.cairn.app

import android.app.Activity
import android.content.Context
import android.net.Uri
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivityResultRegistryOwner
import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.ActivityResultRegistryOwner
import androidx.activity.result.contract.ActivityResultContract
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import androidx.core.app.ActivityOptionsCompat
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.DeckSelector
import dev.cairn.ffi.FileArrival
import dev.cairn.ffi.FilePreview
import dev.cairn.ffi.ImportGate
import dev.cairn.ffi.ThemeChoice
import dev.cairn.ffi.benchFiles
import kotlinx.coroutines.CompletableDeferred
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
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference

/**
 * *Import* writes the plan and lands on the note list, saying nothing (#243, ADR-0041 §3, ADR-0022
 * §5): filtered to the file's deck, or unfiltered for several, with Review under it on the back stack.
 * A write the store refuses is said on the preview, and the gate stays.
 */
@RunWith(AndroidJUnit4::class)
class ImportLandingTest {
    @get:Rule
    val rule = createAndroidComposeRule<ComponentActivity>()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "landing-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    // The bench's files in the order the set builds them.
    private val bench = benchFiles(context.cacheDir.path)
    private val update = bench[0]
    private val several = bench[1]

    /**
     * The real collection, except that the call numbered [failAt] is refused the way a store fault
     * is, and that while [held] is set every call waits for it — a write still in flight.
     */
    private class Access(private val real: CollectionOwner) : CollectionAccess {
        val calls = AtomicInteger()

        @Volatile
        var failAt = -1

        @Volatile
        var held: CompletableDeferred<Unit>? = null

        override suspend fun <T> use(block: (CollectionHandle) -> T): T {
            held?.await()
            if (calls.incrementAndGet() == failAt) throw CairnException.Failed("refused")
            return real.use(block)
        }
    }

    private val access = Access(owner)

    private val chosen = AtomicReference<Uri?>(null)

    private val picker = object : ActivityResultRegistryOwner {
        override val activityResultRegistry = object : ActivityResultRegistry() {
            override fun <I, O> onLaunch(requestCode: Int, contract: ActivityResultContract<I, O>, input: I, options: ActivityOptionsCompat?) {
                dispatchResult(requestCode, Activity.RESULT_OK, android.content.Intent().setData(chosen.get()))
            }
        }
    }

    @Before
    fun serve() {
        for (file in bench) BenchProvider.files[file.name] = file.bytes
    }

    @After
    fun tearDown() {
        BenchProvider.files.clear()
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun selected(tag: String): Boolean =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config.getOrNull(SemanticsProperties.Selected) == true

    private fun showApp() {
        val appearance = AppearanceState(ThemeChoice.LIGHT)
        rule.setContent {
            CompositionLocalProvider(LocalActivityResultRegistryOwner provides picker) {
                CairnAppearance(appearance) { CairnNavHost(access, keyboardUp = false) }
            }
        }
        waitFor("destinations")
    }

    private fun pick(name: String) {
        chosen.set(BenchProvider.uri(name))
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("open-file")
        rule.onNodeWithTag("open-file").performClick()
        waitFor("gate-import")
    }

    private fun pressBack() {
        InstrumentationRegistry.getInstrumentation().runOnMainSync { rule.activity.onBackPressedDispatcher.onBackPressed() }
        rule.waitForIdle()
    }

    /** What the same file would do now, read straight from the store. */
    private fun gateNow(): ImportGate = runBlocking {
        when (val preview = owner.use { it.openFile(FileArrival.PICKED, null, update.bytes, true).preview() }) {
            is FilePreview.Plan -> preview.plan.gate
            is FilePreview.Refused -> preview.gate
            is FilePreview.Restore -> preview.restore.gate
        }
    }

    private fun landOnNotes() {
        rule.onNodeWithTag("gate-import").performClick()
        waitFor("deck-filter")
    }

    @Test
    fun import_lands_on_the_note_list_filtered_to_the_update_deck_saying_nothing() {
        fixture("decks")
        showApp()
        pick(update.name)
        landOnNotes()
        assertFalse(exists("import-screen"))
        assertTrue(exists("destinations"))
        assertTrue(selected("destination-notes"))
        rule.onNodeWithTag("deck-filter").assertTextEquals("French A1")
        rule.onNodeWithTag("notes-search").assertTextEquals("")
        // Nothing is said: no failure, no count, no message to dismiss.
        assertFalse(exists("import-failure"))
        // It was written: the same file now changes nothing.
        assertEquals(ImportGate.DISMISS, gateNow())
    }

    @Test
    fun the_three_deck_file_lands_on_the_note_list_unfiltered() {
        fixture("decks")
        showApp()
        pick(several.name)
        landOnNotes()
        rule.onNodeWithTag("deck-filter").assertTextEquals("All decks")
        assertTrue(selected("destination-notes"))
        assertTrue(runBlocking { owner.use { it.noteList(DeckSelector.All, "") } }.decks.any { it.name == "Deutsch" })
    }

    @Test
    fun back_from_the_landing_goes_to_review() {
        fixture("decks")
        showApp()
        pick(update.name)
        landOnNotes()
        pressBack()
        rule.waitUntil(10_000) { !exists("deck-filter") }
        assertTrue(selected("destination-review"))
        assertFalse(exists("import-screen"))
    }

    @Test
    fun a_refused_write_is_said_on_the_preview_and_the_gate_stays() {
        fixture("decks")
        showApp()
        pick(update.name)
        // The next call is the write.
        access.failAt = access.calls.get() + 1
        rule.onNodeWithTag("gate-import").performClick()
        waitFor("import-failure")
        assertEquals("Could not write the import: refused", text("import-failure"))
        assertTrue(exists("import-screen"))
        rule.onNodeWithTag("gate-import").assertIsEnabled()
        rule.onNodeWithTag("gate-cancel").assertIsEnabled()
        assertFalse(exists("gate-close"))
        // The plan is read again, and nothing was written.
        assertTrue(exists("import-deck-0"))
        assertEquals(ImportGate.DECIDE, gateNow())
    }

    @Test
    fun the_gate_is_disabled_while_the_write_is_in_flight() {
        fixture("decks")
        showApp()
        pick(update.name)
        val write = CompletableDeferred<Unit>()
        access.held = write
        rule.onNodeWithTag("gate-import").performClick()
        rule.waitForIdle()
        rule.onNodeWithTag("gate-import").assertIsNotEnabled()
        rule.onNodeWithTag("gate-cancel").assertIsNotEnabled()
        access.held = null
        write.complete(Unit)
        waitFor("deck-filter")
        assertTrue(selected("destination-notes"))
    }

    @Test
    fun back_while_the_write_is_in_flight_does_nothing_and_the_import_lands() {
        fixture("decks")
        showApp()
        pick(update.name)
        val write = CompletableDeferred<Unit>()
        access.held = write
        rule.onNodeWithTag("gate-import").performClick()
        rule.waitForIdle()
        // Back is the gate's dismissal, and the gate cannot be pressed now.
        pressBack()
        assertTrue(exists("import-screen"))
        access.held = null
        write.complete(Unit)
        waitFor("deck-filter")
        rule.onNodeWithTag("deck-filter").assertTextEquals("French A1")
    }

    @Test
    fun the_landing_clears_a_search_and_a_filter_the_note_list_had() {
        fixture("decks")
        showApp()
        rule.onNodeWithTag("destination-notes").performClick()
        waitFor("notes-search")
        rule.onNodeWithTag("notes-search").performTextInput("zzz")
        rule.waitForIdle()
        rule.onNodeWithTag("notes-search").assertTextEquals("zzz")
        pick(update.name)
        landOnNotes()
        rule.onNodeWithTag("deck-filter").assertTextEquals("French A1")
        rule.onNodeWithTag("notes-search").assertTextEquals("")
        // The note list left before the import is not restored over the landing later either.
        pressBack()
        rule.waitUntil(10_000) { !exists("deck-filter") }
        rule.onNodeWithTag("destination-notes").performClick()
        waitFor("notes-search")
        rule.onNodeWithTag("notes-search").assertTextEquals("")
    }

    @Test
    fun a_sitting_in_progress_is_still_there_after_an_import() {
        fixture("backlog")
        showApp()
        waitFor("shorter-5")
        rule.onNodeWithTag("shorter-5").performClick()
        waitFor("card")
        rule.onNodeWithTag("card").performClick()
        waitFor("grades")
        rule.onNodeWithTag("grade-good").performScrollTo().performClick()
        rule.waitUntil(10_000) { exists("progress") && text("progress") == "1 of 5" }

        pick(update.name)
        landOnNotes()
        pressBack()
        waitFor("progress")
        // Review reads its queue again on return, and the sitting goes on where it was.
        assertEquals("1 of 5", text("progress"))
        assertTrue(exists("card"))
    }
}
