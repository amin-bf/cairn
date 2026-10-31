package dev.cairn.app

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivityResultRegistryOwner
import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.ActivityResultRegistryOwner
import androidx.activity.result.contract.ActivityResultContract
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.core.app.ActivityOptionsCompat
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.FileArrival
import dev.cairn.ffi.FilePreview
import dev.cairn.ffi.ImportGate
import dev.cairn.ffi.RestorePreviewFacts
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
import java.util.TimeZone
import java.util.concurrent.atomic.AtomicReference

/**
 * An opened archive shows the restore preview (#252, ADR-0053 §2, ADR-0016 §10 §11): the bench archive
 * — a backup of `Fixture::Decks` under an id of its own — picked over a collection holding nothing,
 * through the real binding, with the picker stubbed through a test `ActivityResultRegistry`. *Restore*
 * itself writes nothing yet (#253).
 */
@RunWith(AndroidJUnit4::class)
class RestorePreviewTest {
    @get:Rule
    val rule = createAndroidComposeRule<ComponentActivity>()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "restore-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))
    private val archive = benchFiles(context.cacheDir.path)[2]

    /** The bench archive's creation instant, 2026-09-01T00:00:00Z (`file_bench::ARCHIVE_CREATED_MS`). */
    private val createdMs = 20_697L * 86_400_000L

    private val chosen = AtomicReference<Uri?>(null)

    private val picker = object : ActivityResultRegistryOwner {
        override val activityResultRegistry = object : ActivityResultRegistry() {
            override fun <I, O> onLaunch(requestCode: Int, contract: ActivityResultContract<I, O>, input: I, options: ActivityOptionsCompat?) {
                val uri = chosen.get()
                if (uri == null) {
                    dispatchResult(requestCode, Activity.RESULT_CANCELED, null)
                } else {
                    dispatchResult(requestCode, Activity.RESULT_OK, Intent().setData(uri))
                }
            }
        }
    }

    private val zone: TimeZone = TimeZone.getDefault()

    @Before
    fun serve() {
        BenchProvider.files[archive.name] = archive.bytes
        // A fixed zone, so the archive's day is the one its name carries wherever the emulator is set.
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"))
        runBlocking { owner.use { it.installFixture("empty", System.currentTimeMillis()) } }
    }

    @After
    fun tearDown() {
        TimeZone.setDefault(zone)
        BenchProvider.files.clear()
        root.deleteRecursively()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun showApp() {
        val appearance = AppearanceState(ThemeChoice.LIGHT)
        rule.setContent {
            CompositionLocalProvider(LocalActivityResultRegistryOwner provides picker) {
                CairnAppearance(appearance) { CairnNavHost(owner, keyboardUp = false) }
            }
        }
        waitFor("destinations")
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("open-file")
    }

    private fun pick(query: String? = null) {
        chosen.set(BenchProvider.uri(archive.name, query))
        rule.onNodeWithTag("open-file").performClick()
        waitFor("import-screen")
    }

    private fun pressBack() {
        InstrumentationRegistry.getInstrumentation().runOnMainSync { rule.activity.onBackPressedDispatcher.onBackPressed() }
        rule.waitForIdle()
    }

    private fun noteCount() = runBlocking { owner.use { it.noteCount() } }

    @Test
    fun a_picked_archive_states_its_restore_and_offers_restore_and_cancel() {
        showApp()
        pick()
        assertEquals(archive.name, text("import-heading"))
        assertEquals("Collection archive, 1 September 2026. 25 notes, 50 reviews.", text("restore-summary"))
        assertEquals("Restore adds to this collection. It never removes anything you already hold.", text("restore-line"))
        assertEquals("Restore", text("gate-restore"))
        assertTrue(exists("gate-cancel"))
        assertFalse(exists("gate-import") || exists("gate-close"))
        // Nothing of the import preview's: no decks, no history line, no refusal.
        assertFalse(exists("import-deck-0") || exists("import-line") || exists("import-refusal"))
        assertFalse(exists("destinations"))
    }

    @Test
    fun an_archive_with_no_name_is_headed_a_collection_archive() {
        showApp()
        pick(query = "nameless")
        assertEquals("A collection archive", text("import-heading"))
    }

    @Test
    fun cancel_and_back_return_to_settings_with_nothing_written() {
        showApp()
        pick()
        rule.onNodeWithTag("gate-cancel").performClick()
        waitFor("open-file")
        assertFalse(exists("import-screen"))
        assertTrue(exists("destinations"))

        pick()
        pressBack()
        waitFor("open-file")
        assertFalse(exists("import-screen"))
        assertEquals(0uL, noteCount())
    }

    @Test
    fun restore_writes_nothing_yet_and_says_so_on_the_preview() {
        showApp()
        pick()
        rule.onNodeWithTag("gate-restore").performClick()
        waitFor("restore-not-yet")
        // The app's own sentence, not a write failure worded by the binding (#248: the binding sends facts).
        assertEquals("Restoring is not built yet. Nothing was written.", text("restore-not-yet"))
        assertFalse(exists("import-failure"))
        assertTrue(exists("restore-summary"))
        rule.onNodeWithTag("gate-cancel").assertIsEnabled()
        assertEquals(0uL, noteCount())
    }

    @Test
    fun another_collections_archive_is_refused_with_a_lone_close_that_leaves() {
        runBlocking { owner.use { it.installFixture("decks", System.currentTimeMillis()) } }
        val before = noteCount()
        showApp()
        pick()
        assertEquals(archive.name, text("import-heading"))
        assertEquals(
            "This archive is from a different collection. To use it on this device: back up what is here, clear the app's data, restore.",
            text("import-refusal"),
        )
        assertTrue(exists("gate-close"))
        assertFalse(exists("gate-restore") || exists("gate-cancel") || exists("restore-summary"))
        rule.onNodeWithTag("gate-close").performClick()
        waitFor("open-file")
        assertFalse(exists("import-screen"))
        assertEquals(before, noteCount())
    }

    @Test
    fun an_archive_past_the_read_ceiling_is_too_large_to_open() {
        // A head the platform stopped at: what a file past the ceiling hands over.
        val preview = runBlocking {
            owner.use { it.openFile(FileArrival.PICKED, null, archive.bytes.copyOf(512), false).preview() }
        }
        rule.setContent { CairnTheme(false) { ImportPage(null, preview, onImport = {}, onLeave = {}) } }
        assertEquals("A collection archive", text("import-heading"))
        assertEquals("This archive is too large to open on this phone.", text("import-refusal"))
        assertTrue(exists("gate-close"))
    }

    @Test
    fun an_archive_dated_in_a_form_this_app_does_not_write_states_its_counts_alone() {
        val preview = FilePreview.Restore(RestorePreviewFacts(createdMs = null, notes = 1uL, reviews = 4_200uL, gate = ImportGate.DECIDE))
        rule.setContent { CairnTheme(false) { ImportPage("odd.ccoll", preview, onImport = {}, onLeave = {}) } }
        assertEquals("Collection archive. 1 note, 4,200 reviews.", text("restore-summary"))
    }

    @Test
    fun the_archive_day_is_the_local_one_with_its_year() {
        assertEquals("1 September 2026", archiveDate(createdMs, TimeZone.getTimeZone("UTC")))
        // West of UTC the same instant is still the evening before.
        assertEquals("31 August 2026", archiveDate(createdMs, TimeZone.getTimeZone("America/New_York")))
    }

    // --- Screenshots, in both themes.

    private var scene by mutableStateOf<@Composable () -> Unit>({})
    private var darkScene by mutableStateOf(false)

    private fun shoot(name: String, dark: Boolean, content: @Composable () -> Unit) {
        val dir = context.getExternalFilesDir("restore-shots")!!
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
    fun the_restore_preview_and_its_refusal_are_drawn_in_both_themes() {
        val restore = runBlocking { owner.use { it.openFile(FileArrival.PICKED, archive.name, archive.bytes, true).preview() } }
        assertTrue(restore is FilePreview.Restore)
        runBlocking { owner.use { it.installFixture("decks", System.currentTimeMillis()) } }
        val refusal = runBlocking { owner.use { it.openFile(FileArrival.PICKED, archive.name, archive.bytes, true).preview() } }
        assertTrue(refusal is FilePreview.Refused)
        rule.setContent { CairnTheme(darkScene) { scene() } }
        for (dark in listOf(false, true)) {
            shoot("restore", dark) { ImportPage(archive.name, restore, onImport = {}, onLeave = {}) }
            assertTrue(exists("gate-restore"))
            shoot("another-collection", dark) { ImportPage(archive.name, refusal, onImport = {}, onLeave = {}) }
            assertTrue(exists("gate-close"))
        }
    }
}

/** The restore preview through a recreation of the activity: the model holds the archive, so the preview survives a rotation. */
@RunWith(AndroidJUnit4::class)
class RestoreRotationTest {
    @get:Rule
    val rule = createEmptyComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "restore-rotation-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        RotatingActivity.access = null
        RotatingActivity.registry = null
        BenchProvider.files.clear()
        root.deleteRecursively()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    @Test
    fun the_restore_preview_survives_a_rotation() {
        val archive = benchFiles(context.cacheDir.path)[2]
        BenchProvider.files[archive.name] = archive.bytes
        runBlocking { owner.use { it.installFixture("empty", System.currentTimeMillis()) } }
        RotatingActivity.access = owner
        RotatingActivity.registry = object : ActivityResultRegistryOwner {
            override val activityResultRegistry = object : ActivityResultRegistry() {
                override fun <I, O> onLaunch(requestCode: Int, contract: ActivityResultContract<I, O>, input: I, options: ActivityOptionsCompat?) {
                    dispatchResult(requestCode, Activity.RESULT_OK, Intent().setData(BenchProvider.uri(archive.name)))
                }
            }
        }
        ActivityScenario.launch(RotatingActivity::class.java).use { scenario ->
            waitFor("destination-settings")
            rule.onNodeWithTag("destination-settings").performClick()
            waitFor("open-file")
            rule.onNodeWithTag("open-file").performClick()
            waitFor("restore-summary")
            val summary = text("restore-summary")
            scenario.recreate()
            waitFor("restore-summary")
            assertEquals(summary, text("restore-summary"))
            assertEquals(archive.name, text("import-heading"))
            assertTrue(exists("gate-restore"))
        }
    }
}
