package dev.cairn.app

import android.content.ClipData
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.ActivityInfo
import android.content.pm.PackageManager
import android.net.Uri
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.DeckSelector
import dev.cairn.ffi.FileArrival
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

/**
 * *Open with* and share reach the import preview (#244, ADR-0024 §2, ADR-0022 §6, ADR-0052): `VIEW`
 * and `SEND` intents launched at the real activity, cold and while it runs, with the bench's files
 * served by the debug [BenchProvider] over `Fixture::Decks` in the app's own collection.
 */
@RunWith(AndroidJUnit4::class)
class FileIntentTest {
    @get:Rule
    val rule = createEmptyComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val app = context as CairnApplication

    // The bench's files in the order the set builds them.
    private val bench = benchFiles(context.cacheDir.path)
    private val update = bench[0]
    private val several = bench[1]
    private val archive = bench[2]

    @Before
    fun serve() {
        for (file in bench) BenchProvider.files[file.name] = file.bytes
        runBlocking { app.collection.use { it.installFixture("decks", System.currentTimeMillis()) } }
    }

    @After
    fun tearDown() {
        HandledArrivals.of(context).clear()
        BenchProvider.files.clear()
        BenchProvider.opened.clear()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    // Longer than the suite's 10 s: each test starts the real activity, cold, and reads a file through it.
    private fun waitUntil(condition: () -> Boolean) = rule.waitUntil(30_000, condition)

    private fun waitFor(tag: String) = waitUntil { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun selected(tag: String): Boolean =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Selected]

    /** Review's entrance: the destination is Review and no preview is up. */
    private fun onReviewsEntrance(): Boolean =
        exists("destinations") && selected("destination-review") && !exists("import-screen") &&
            (exists("start-all") || exists("state-caught-up"))

    /** An intent at the activity, as the system delivers one another app sent. */
    private fun toActivity(action: String) = Intent(action).setClass(context, MainActivity::class.java)

    private fun view(uri: Uri) = toActivity(Intent.ACTION_VIEW).setDataAndType(uri, "application/octet-stream")

    private fun sendStream(uri: Uri) =
        toActivity(Intent.ACTION_SEND).setType("application/octet-stream").putExtra(Intent.EXTRA_STREAM, uri)

    private fun sendClip(uri: Uri) =
        toActivity(Intent.ACTION_SEND).setType("application/octet-stream").also { it.clipData = ClipData.newRawUri(null, uri) }

    /** Review's entrance stays up: a second opening has every chance to show itself, and does not. */
    private fun staysOnReviewsEntrance() {
        waitUntil { onReviewsEntrance() }
        Thread.sleep(1_000)
        rule.waitForIdle()
        assertTrue(onReviewsEntrance())
    }

    /**
     * A file reaches the running app: singleTask brings the intent to `onNewIntent` rather than a second
     * copy. It is started from the activity itself, since a start from the test's context with
     * `NEW_TASK` leaves the scenario unable to close it.
     */
    private fun arriveWhileRunning(scenario: ActivityScenario<MainActivity>, intent: Intent) {
        scenario.onActivity { it.startActivity(intent) }
    }

    private fun pressBack(scenario: ActivityScenario<MainActivity>) {
        scenario.onActivity { it.onBackPressedDispatcher.onBackPressed() }
        rule.waitForIdle()
    }

    // --- The manifest (ADR-0024 §2).

    @Test
    fun the_merged_manifest_offers_the_app_for_both_types_by_view_and_send() {
        val pm = context.packageManager
        val ours = MainActivity::class.java.name
        val uri = Uri.parse("content://elsewhere/deck")
        for (type in listOf("application/octet-stream", "application/vnd.cairn.deck+zip")) {
            val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, type).setPackage(context.packageName)
            val browsable = Intent(view).addCategory(Intent.CATEGORY_BROWSABLE)
            val send = Intent(Intent.ACTION_SEND).setType(type).setPackage(context.packageName)
            for ((what, intent) in listOf("VIEW" to view, "VIEW, browsable" to browsable, "SEND" to send)) {
                val found = pm.queryIntentActivities(intent, PackageManager.MATCH_DEFAULT_ONLY or PackageManager.GET_RESOLVED_FILTER)
                val match = found.firstOrNull { it.activityInfo.name == ours }
                assertTrue("$what for $type reaches the activity: ${found.map { it.activityInfo.name }}", match != null)
                // The filter's own declaration: its type, DEFAULT, and BROWSABLE on VIEW alone.
                val filter = match!!.filter
                assertTrue("$what's filter names $type", filter.hasDataType(type))
                assertTrue("$what's filter is DEFAULT", filter.hasCategory(Intent.CATEGORY_DEFAULT))
                assertEquals("$what's filter is BROWSABLE", intent.action == Intent.ACTION_VIEW, filter.hasCategory(Intent.CATEGORY_BROWSABLE))
            }
        }
        val info = pm.getActivityInfo(ComponentName(context, MainActivity::class.java), 0)
        assertEquals(ActivityInfo.LAUNCH_SINGLE_TASK, info.launchMode)
    }

    // --- Arriving.

    @Test
    fun a_cold_start_reaches_the_preview_and_cancel_lands_on_reviews_entrance() {
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(update.name))).use {
            waitFor("import-screen")
            waitFor("gate-cancel")
            assertEquals(update.name, text("import-heading"))
            assertFalse("the preview takes the whole screen", exists("destinations"))
            rule.onNodeWithTag("gate-cancel").performClick()
            waitUntil { onReviewsEntrance() }
        }
    }

    // --- An intent is acted on once (#255): the system replays a task's root intent after an upgrade.

    /** The file was opened under an earlier build: the record says so, as the one the last run left. */
    private fun openedUnderAnEarlierBuild(file: ArrivingFile) =
        HandledArrivals.of(context, installed = { 1L }).handled(file)

    @Test
    fun a_cold_start_replaying_the_file_opened_before_an_upgrade_reaches_no_preview() {
        openedUnderAnEarlierBuild(ArrivingFile(FileArrival.OPENED, BenchProvider.uri(update.name)))
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(update.name))).use {
            staysOnReviewsEntrance()
            assertFalse(exists("import-screen"))
        }
    }

    @Test
    fun a_replay_is_dropped_once_and_the_same_file_opened_again_reaches_the_preview() {
        openedUnderAnEarlierBuild(ArrivingFile(FileArrival.OPENED, BenchProvider.uri(update.name)))
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(update.name))).use { scenario ->
            staysOnReviewsEntrance()
            arriveWhileRunning(scenario, view(BenchProvider.uri(update.name)))
            waitFor("import-screen")
            assertEquals(update.name, text("import-heading"))
        }
    }

    @Test
    fun a_running_app_gets_a_replay_dropped_and_another_file_still_arrives() {
        openedUnderAnEarlierBuild(ArrivingFile(FileArrival.OPENED, BenchProvider.uri(update.name)))
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            waitUntil { onReviewsEntrance() }
            arriveWhileRunning(scenario, view(BenchProvider.uri(update.name)))
            staysOnReviewsEntrance()
            arriveWhileRunning(scenario, view(BenchProvider.uri(several.name)))
            waitFor("import-screen")
            assertEquals(several.name, text("import-heading"))
        }
    }

    @Test
    fun a_file_opened_under_this_build_is_not_taken_for_a_replay() {
        HandledArrivals.of(context).handled(ArrivingFile(FileArrival.OPENED, BenchProvider.uri(update.name)))
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(update.name))).use {
            waitFor("import-screen")
            assertEquals(update.name, text("import-heading"))
        }
    }

    @Test
    fun a_running_app_reaches_the_preview_from_notes_and_back_returns_there() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            waitFor("destination-notes")
            rule.onNodeWithTag("destination-notes").performClick()
            waitFor("notes-search")
            arriveWhileRunning(scenario, view(BenchProvider.uri(update.name)))
            waitFor("import-screen")
            waitFor("gate-cancel")
            assertEquals(update.name, text("import-heading"))
            pressBack(scenario)
            waitFor("notes-search")
            assertFalse(exists("import-screen"))
            assertTrue(selected("destination-notes"))
        }
    }

    @Test
    fun an_open_editors_edit_is_settled_before_the_preview_takes_the_screen() {
        val note = runBlocking { app.collection.use { it.noteList(DeckSelector.All, "").rows } }[0]
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            waitFor("destination-notes")
            rule.onNodeWithTag("destination-notes").performClick()
            waitFor("note-row")
            rule.onAllNodes(hasTestTag("note-row"))[0].performClick()
            waitFor("field-Front")
            rule.onNodeWithTag("field-Front").performTextInput("zzz")
            arriveWhileRunning(scenario, view(BenchProvider.uri(update.name)))
            waitFor("import-screen")
            val stored = runBlocking {
                app.collection.use { h -> h.editNote(note.noteId).fields().first { it.name == "Front" }.value }
            }
            assertTrue("the edit is in the store when the preview is up: $stored", "zzz" in stored)
        }
    }

    @Test
    fun an_open_editors_edit_is_settled_before_a_restore_preview_and_the_restore_keeps_it() {
        val note = runBlocking { app.collection.use { it.noteList(DeckSelector.All, "").rows } }[0]
        // The collection's own archive, made before the edit: its note is older than the edit.
        val own = runBlocking { app.collection.use { it.backup(System.currentTimeMillis(), 0) } }
        BenchProvider.files[own.name] = own.bytes
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            waitFor("destination-notes")
            rule.onNodeWithTag("destination-notes").performClick()
            waitFor("note-row")
            rule.onAllNodes(hasTestTag("note-row"))[0].performClick()
            waitFor("field-Front")
            rule.onNodeWithTag("field-Front").performTextInput("zzz")
            arriveWhileRunning(scenario, view(BenchProvider.uri(own.name)))
            waitFor("gate-restore")
            fun stored() = runBlocking {
                app.collection.use { h -> h.editNote(note.noteId).fields().first { it.name == "Front" }.value }
            }
            assertTrue("the edit is in the store when the preview is up: ${stored()}", "zzz" in stored())
            rule.onNodeWithTag("gate-restore").performClick()
            waitFor("deck-filter")
            assertTrue("the edit outlives the restore: ${stored()}", "zzz" in stored())
        }
    }

    @Test
    fun a_second_file_replaces_the_first_and_back_returns_to_where_the_person_was() {
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(update.name))).use { scenario ->
            waitFor("import-screen")
            waitUntil { exists("import-heading") && text("import-heading") == update.name }
            arriveWhileRunning(scenario, view(BenchProvider.uri(several.name)))
            waitUntil { text("import-heading") == several.name }
            pressBack(scenario)
            // One preview was up, not two: Back leaves for Review's entrance.
            waitUntil { onReviewsEntrance() }
        }
    }

    @Test
    fun a_send_carrying_its_file_in_the_stream_extra_reaches_the_preview() {
        ActivityScenario.launch<MainActivity>(sendStream(BenchProvider.uri(update.name))).use {
            waitFor("import-screen")
            waitFor("import-deck-0")
            assertEquals(update.name, text("import-heading"))
        }
    }

    /** The app's collection holding nothing, so the bench archive is one this device may restore. */
    private fun emptied() = runBlocking { app.collection.use { it.installFixture("empty", System.currentTimeMillis()) } }

    @Test
    fun an_archive_opened_with_the_app_reaches_the_restore_preview() {
        emptied()
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(archive.name))).use {
            waitFor("restore-summary")
            assertEquals(archive.name, text("import-heading"))
            assertTrue(exists("gate-restore") && exists("gate-cancel"))
        }
    }

    @Test
    fun an_archive_shared_to_the_app_reaches_the_restore_preview() {
        emptied()
        ActivityScenario.launch<MainActivity>(sendStream(BenchProvider.uri(archive.name))).use {
            waitFor("restore-summary")
            assertEquals(archive.name, text("import-heading"))
            assertTrue(exists("gate-restore") && exists("gate-cancel"))
        }
    }

    @Test
    fun a_send_carrying_only_clip_data_reaches_the_preview() {
        ActivityScenario.launch<MainActivity>(sendClip(BenchProvider.uri(several.name))).use {
            waitFor("import-screen")
            waitFor("import-deck-0")
            assertEquals(several.name, text("import-heading"))
        }
    }

    @Test
    fun an_intent_is_not_opened_again_across_a_rotation() {
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(update.name))).use { scenario ->
            waitFor("gate-cancel")
            rule.onNodeWithTag("gate-cancel").performClick()
            waitUntil { onReviewsEntrance() }
            scenario.recreate()
            staysOnReviewsEntrance()
        }
    }

    @Test
    fun a_rotation_mid_read_reads_the_document_once() {
        ActivityScenario.launch<MainActivity>(view(BenchProvider.uri(update.name, "slow"))).use { scenario ->
            waitUntil { BenchProvider.openings(update.name) == 1 }
            scenario.recreate()
            waitFor("import-deck-0")
            assertEquals(update.name, text("import-heading"))
            assertEquals("the document was read once", 1, BenchProvider.openings(update.name))
        }
    }

    @Test
    fun an_intent_brought_back_from_recents_is_not_opened_again() {
        val again = view(BenchProvider.uri(update.name)).addFlags(Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY)
        ActivityScenario.launch<MainActivity>(again).use { staysOnReviewsEntrance() }
    }
}
