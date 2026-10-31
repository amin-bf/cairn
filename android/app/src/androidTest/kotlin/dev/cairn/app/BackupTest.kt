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
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.core.app.ActivityOptionsCompat
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.FileArrival
import dev.cairn.ffi.FilePreview
import dev.cairn.ffi.FileRefusal
import dev.cairn.ffi.ThemeChoice
import kotlinx.coroutines.runBlocking
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.FileOutputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import java.util.zip.ZipInputStream

/**
 * *Back up…* on Settings (#249, ADR-0053 §4, ADR-0016 §6): the nudge, the save dialog stubbed through a
 * test `ActivityResultRegistry`, the archive written into the debug `BenchProvider`, and the report in
 * the row — over the history fixture, through the real binding.
 */
@RunWith(AndroidJUnit4::class)
class BackupTest {
    @get:Rule
    val rule = createAndroidComposeRule<ComponentActivity>()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "backup-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    /** What the stubbed save dialog answers with; null is a dialog closed with nothing. */
    private val chosen = AtomicReference<Uri?>(null)

    /** The name the app asked the dialog to suggest. */
    private val asked = AtomicReference<Any?>(null)

    /** Whether the dialog answers at once, or holds its answer until the test hands it over. */
    @Volatile
    private var holding = false

    private val pending = AtomicInteger(-1)

    /** Every intent the app started through the registry other than the save dialog: the share sheet's chooser. */
    private val started: MutableList<Intent> = java.util.concurrent.CopyOnWriteArrayList()

    private val dialog = object : ActivityResultRegistryOwner {
        override val activityResultRegistry = object : ActivityResultRegistry() {
            override fun <I, O> onLaunch(requestCode: Int, contract: ActivityResultContract<I, O>, input: I, options: ActivityOptionsCompat?) {
                if (input is Intent) {
                    started += input
                    return
                }
                asked.set(input)
                if (holding) pending.set(requestCode) else answer(requestCode)
            }

            fun answer(requestCode: Int) {
                val uri = chosen.get()
                if (uri == null) {
                    dispatchResult(requestCode, Activity.RESULT_CANCELED, null)
                } else {
                    dispatchResult(requestCode, Activity.RESULT_OK, Intent().setData(uri))
                }
            }
        }
    }

    /** Hand over the answer a holding dialog kept back. */
    private fun answerHeld() {
        val registry = dialog.activityResultRegistry
        rule.runOnUiThread {
            val uri = chosen.get()
            if (uri == null) {
                registry.dispatchResult(pending.get(), Activity.RESULT_CANCELED, null)
            } else {
                registry.dispatchResult(pending.get(), Activity.RESULT_OK, Intent().setData(uri))
            }
        }
    }

    @Before
    fun withHistory() {
        BenchProvider.files.clear()
        BenchProvider.deleted.clear()
        runBlocking { owner.use { it.installFixture("history", System.currentTimeMillis()) } }
    }

    @After
    fun tearDown() {
        BenchProvider.files.clear()
        BenchProvider.deleted.clear()
        root.deleteRecursively()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun showApp() {
        val appearance = AppearanceState(ThemeChoice.LIGHT)
        rule.setContent {
            CompositionLocalProvider(LocalActivityResultRegistryOwner provides dialog) {
                CairnAppearance(appearance) { CairnNavHost(owner, keyboardUp = false) }
            }
        }
        waitFor("destinations")
    }

    private fun openSettings() {
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("backup-nudge")
    }

    private fun backUpTo(uri: Uri?) {
        chosen.set(uri)
        rule.onNodeWithTag("back-up").performScrollTo().performClick()
    }

    private val noBackupYet = "No backup yet. 225 reviews across 25 notes."

    private fun today(): String = SimpleDateFormat("d MMMM", Locale.ENGLISH).format(Date())

    private fun suggested(ms: Long): String =
        "Cairn backup ${SimpleDateFormat("yyyy-MM-dd", Locale.ENGLISH).format(Date(ms))}.ccoll"

    private fun lastBackup() = runBlocking { owner.use { it.backupNudge().lastBackupMs } }

    // --- The nudge (ADR-0016 §6).

    @Test
    fun the_nudge_before_any_backup_counts_every_review_across_the_notes() {
        showApp()
        openSettings()
        assertEquals("Back up…", text("back-up"))
        assertEquals(noBackupYet, text("backup-nudge"))
        // Back up… sits under Open a file… and its line, the nudge under it.
        assertTrue(top("files-line") < top("back-up"))
        assertTrue(top("back-up") < top("backup-nudge"))
        assertTrue(top("backup-nudge") < top("label-Appearance"))
    }

    /** Where a node starts down the page, scrolled out of view or not (`boundsInRoot` is clipped to the viewport). */
    private fun top(tag: String): Float = rule.onNodeWithTag(tag).fetchSemanticsNode().positionInRoot.y

    @Test
    fun both_nudges_group_thousands_and_agree_their_nouns() {
        val states = listOf(
            BackupNudgeShown.None(reviews = 4200, notes = 812),
            BackupNudgeShown.None(reviews = 1, notes = 1),
            BackupNudgeShown.Last(date = "3 March", reviewsSince = 1240),
            BackupNudgeShown.Last(date = "3 March 2025", reviewsSince = 1),
        )
        var shown by mutableStateOf(states.first())
        rule.setContent {
            CairnTheme(dark = false) { SettingsPage(SettingsScreenState.Ready("5", ThemeChoice.LIGHT, backupNudge = shown), {}, {}, {}, {}, {}) }
        }
        val said = states.map {
            shown = it
            rule.waitForIdle()
            text("backup-nudge")
        }
        assertEquals(
            listOf(
                "No backup yet. 4,200 reviews across 812 notes.",
                "No backup yet. 1 review across 1 note.",
                "Last backup 3 March. 1,240 reviews since.",
                "Last backup 3 March 2025. 1 review since.",
            ),
            said,
        )
    }

    @Test
    fun the_date_is_the_day_and_the_month_with_the_year_only_when_it_is_not_this_year() {
        val utc = TimeZone.getTimeZone("UTC")
        // 2026-03-03T09:00Z, seen on 2026-10-31 and on 2027-01-02.
        val march = 1_772_528_400_000L
        assertEquals("3 March", backupDate(march, 1_793_440_000_000L, utc))
        assertEquals("3 March 2026", backupDate(march, 1_798_848_000_000L, utc))
        // The day is the zone's: late on the 2nd in UTC is the 3rd an hour east of it.
        val late = 1_772_494_200_000L
        assertEquals("2 March", backupDate(late, march, utc))
        assertEquals("3 March", backupDate(late, march, TimeZone.getTimeZone("GMT+01:00")))
    }

    // --- The flow (ADR-0053 §4).

    @Test
    fun a_backup_writes_an_archive_the_binding_reads_and_the_row_reports_the_providers_name() {
        showApp()
        openSettings()
        val before = System.currentTimeMillis()
        backUpTo(BenchProvider.uri("Bench backup (2).ccoll"))
        waitFor("backup-report")
        val after = System.currentTimeMillis()

        // The dialog opened with the dated name, and the provider's own name is the one reported.
        assertEquals(suggested(before), asked.get())
        assertEquals("Bench backup (2).ccoll — 25 notes, 225 reviews", text("backup-report"))

        // The bytes are a collection archive whose manifest was made at the instant the backup began.
        rule.waitUntil(10_000) { BenchProvider.files.containsKey("Bench backup (2).ccoll") }
        val bytes = BenchProvider.files.getValue("Bench backup (2).ccoll")
        val manifest = manifestOf(bytes)
        assertEquals("collection", manifest.getString("profile"))
        assertEquals(25, manifest.getInt("notes"))
        assertEquals(225, manifest.getInt("reviews"))
        val created = isoMillis(manifest.getString("created"))
        assertTrue("$created in [$before, $after]", created in before..after)
        val preview = runBlocking { owner.use { it.openFile(FileArrival.OPENED, "Bench backup (2).ccoll", bytes, true).preview() } }
        assertEquals(FileRefusal.CollectionArchive, (preview as FilePreview.Refused).refusal)

        // Recorded only now, at that instant, and the nudge moves to today.
        assertEquals(created, lastBackup())
        rule.waitUntil(10_000) { text("backup-nudge") == "Last backup ${today()}. 0 reviews since." }
        assertTrue(BenchProvider.deleted.isEmpty())
    }

    @Test
    fun a_right_to_left_name_is_isolated_in_the_report() {
        showApp()
        openSettings()
        val persian = "پشتیبان.ccoll"
        backUpTo(BenchProvider.uri(persian))
        waitFor("backup-report")
        val wrapped = android.text.BidiFormatter.getInstance().unicodeWrap(persian)
        assertNotEquals("the name is isolated", persian, wrapped)
        assertEquals("$wrapped — 25 notes, 225 reviews", text("backup-report"))
    }

    @Test
    fun a_document_the_provider_gives_no_name_is_reported_without_one_never_with_the_suggested_name() {
        showApp()
        openSettings()
        backUpTo(BenchProvider.uri("Unnamed.ccoll", "nameless"))
        waitFor("backup-report")
        assertEquals("Backed up — 25 notes, 225 reviews", text("backup-report"))
        assertNotNull(lastBackup())
    }

    @Test
    fun a_dialog_closed_with_nothing_does_nothing() {
        showApp()
        openSettings()
        backUpTo(null)
        rule.waitForIdle()
        assertEquals(noBackupYet, text("backup-nudge"))
        assertFalse(exists("backup-report") || exists("backup-failed"))
        assertEquals("Back up…", text("back-up"))
        assertNull(lastBackup())
        assertTrue(BenchProvider.files.isEmpty())
    }

    @Test
    fun a_write_the_provider_fails_is_said_in_the_row_records_nothing_and_deletes_the_document() {
        showApp()
        openSettings()
        val failing = BenchProvider.uri("Refusing.ccoll", "fails")
        backUpTo(failing)
        waitFor("backup-failed")
        assertEquals("Could not write the backup: refused", text("backup-failed"))
        assertEquals(noBackupYet, text("backup-nudge"))
        assertNull(lastBackup())
        assertTrue(failing in BenchProvider.deleted)
        assertFalse(exists("backup-report"))
    }

    @Test
    fun while_the_backup_runs_the_action_is_disabled_and_reads_backing_up() {
        showApp()
        openSettings()
        backUpTo(BenchProvider.uri("Slow.ccoll", "slow"))
        rule.waitUntil(10_000) { text("back-up") == "Backing up…" }
        rule.onNodeWithTag("back-up").assertIsNotEnabled()
        waitFor("backup-report")
        assertEquals("Back up…", text("back-up"))
    }

    @Test
    fun the_report_stays_until_settings_is_left_or_the_next_backup_starts() {
        showApp()
        openSettings()
        backUpTo(BenchProvider.uri("First.ccoll"))
        waitFor("backup-report")
        // A dialog closed with nothing has not started a backup: the report stays.
        backUpTo(null)
        rule.waitForIdle()
        assertTrue(exists("backup-report"))
        // The next backup that starts clears it, and a failure takes its place.
        backUpTo(BenchProvider.uri("Second.ccoll", "fails"))
        waitFor("backup-failed")
        assertFalse(exists("backup-report"))
        // Leaving Settings clears the row.
        rule.onNodeWithTag("destination-review").performClick()
        rule.waitForIdle()
        openSettings()
        assertFalse(exists("backup-report") || exists("backup-failed"))
    }

    @Test
    fun a_process_death_behind_the_dialog_keeps_the_instant() {
        holding = true
        // The models live in a store the test can throw away, as a killed process does; the saved
        // state outlives it.
        var store = ViewModelStore()
        val stores = object : ViewModelStoreOwner {
            override val viewModelStore: ViewModelStore get() = store
        }
        val restoration = StateRestorationTester(rule)
        val appearance = AppearanceState(ThemeChoice.LIGHT)
        restoration.setContent {
            CompositionLocalProvider(LocalActivityResultRegistryOwner provides dialog, LocalViewModelStoreOwner provides stores) {
                CairnAppearance(appearance) { CairnNavHost(owner, keyboardUp = false) }
            }
        }
        waitFor("destinations")
        openSettings()
        val before = System.currentTimeMillis()
        backUpTo(BenchProvider.uri("Behind the dialog.ccoll"))
        rule.waitUntil(10_000) { pending.get() >= 0 }
        val launched = System.currentTimeMillis()
        Thread.sleep(50)

        // A killed process drops its models without clearing them: clearing would run the place's
        // onCleared, which forgets its saved composition state as a pop does, and with it the
        // dialog's registration.
        store = ViewModelStore()
        restoration.emulateSavedInstanceStateRestore()
        waitFor("backup-nudge")
        answerHeld()
        waitFor("backup-report")
        rule.waitUntil(10_000) { BenchProvider.files.containsKey("Behind the dialog.ccoll") }
        val created = isoMillis(manifestOf(BenchProvider.files.getValue("Behind the dialog.ccoll")).getString("created"))
        assertTrue("$created in [$before, $launched]", created in before..launched)
        assertEquals(created, lastBackup())
    }

    // --- Share (ADR-0023 §3 §5 §7, ADR-0047 §4).

    @Test
    fun share_hands_the_saved_document_to_a_chooser_that_carries_the_stream_and_the_grant() {
        showApp()
        openSettings()
        val document = BenchProvider.uri("Shared.ccoll")
        backUpTo(document)
        waitFor("backup-report")
        assertEquals("Share", text("backup-share"))
        rule.onNodeWithTag("backup-share").performScrollTo().performClick()
        rule.waitUntil(10_000) { started.isNotEmpty() }

        val chooser = started.single()
        assertEquals(Intent.ACTION_CHOOSER, chooser.action)
        // The grant and the URI as ClipData are on the chooser itself: createChooser inherits neither.
        assertTrue(chooser.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION != 0)
        assertEquals(document, chooser.clipData!!.getItemAt(0).uri)
        @Suppress("DEPRECATION")
        val send = chooser.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)!!
        assertEquals(Intent.ACTION_SEND, send.action)
        assertEquals("application/octet-stream", send.type)
        @Suppress("DEPRECATION")
        assertEquals(document, send.getParcelableExtra<Uri>(Intent.EXTRA_STREAM))
    }

    @Test
    fun nothing_launches_when_a_backup_finishes_before_share_is_pressed() {
        showApp()
        openSettings()
        backUpTo(BenchProvider.uri("Unshared.ccoll"))
        waitFor("backup-report")
        rule.waitUntil(10_000) { text("backup-nudge").startsWith("Last backup") }
        rule.waitForIdle()
        assertTrue(exists("backup-share"))
        assertTrue("nothing was started: $started", started.isEmpty())
    }

    @Test
    fun share_is_absent_without_a_report() {
        showApp()
        openSettings()
        assertFalse(exists("backup-share"))
        // A write that failed leaves no document to hand on.
        backUpTo(BenchProvider.uri("Refusing.ccoll", "fails"))
        waitFor("backup-failed")
        assertFalse(exists("backup-share"))
        // Settings left and come back to: the report is gone, and Share with it.
        backUpTo(BenchProvider.uri("Kept.ccoll"))
        waitFor("backup-share")
        rule.onNodeWithTag("destination-review").performClick()
        rule.waitForIdle()
        openSettings()
        assertFalse(exists("backup-share"))
        assertTrue(started.isEmpty())
    }

    @Test
    fun a_written_archive_whose_recording_fails_is_still_reported_with_share_and_says_it_was_not_recorded() {
        // The real collection, except that the call after the write, recording it, is refused.
        val refusing = object : CollectionAccess {
            @Volatile
            var refuseNext = false

            override suspend fun <T> use(block: (CollectionHandle) -> T): T {
                if (refuseNext) {
                    refuseNext = false
                    throw CairnException.Failed("disk full")
                }
                return owner.use(block)
            }
        }
        val documents = object : DocumentWriter {
            val real = ResolverDocuments(context.contentResolver)

            override suspend fun write(uri: Uri, bytes: ByteArray): String? = real.write(uri, bytes).also { refusing.refuseNext = true }

            override suspend fun delete(uri: Uri) = real.delete(uri)
        }
        val model = SettingsViewModel(owner = refusing, appearance = AppearanceState(ThemeChoice.LIGHT), documents = documents)
        rule.setContent {
            CompositionLocalProvider(LocalActivityResultRegistryOwner provides dialog) {
                CairnTheme(dark = false) { SettingsScreen(model) }
            }
        }
        waitFor("backup-nudge")
        val document = BenchProvider.uri("Unrecorded.ccoll")
        rule.runOnUiThread { model.backUp(document) }
        waitFor("backup-report")

        // The archive is whole and kept: it is reported, Share hands it on, and the row says what failed.
        assertEquals("Unrecorded.ccoll — 25 notes, 225 reviews", text("backup-report"))
        assertTrue(exists("backup-share"))
        assertEquals("Saved, but not recorded as the last backup: disk full", text("backup-unrecorded"))
        assertFalse(exists("backup-failed"))
        assertTrue(BenchProvider.files.containsKey("Unrecorded.ccoll"))
        assertTrue(BenchProvider.deleted.isEmpty())
        // Nothing was recorded, so the nudge still says what it said.
        assertNull(lastBackup())
        assertEquals(noBackupYet, text("backup-nudge"))
    }

    // --- The Files block, drawn (ADR-0053 §4).

    @Test
    fun the_files_block_is_captured_with_each_nudge_and_with_a_report_in_both_themes() {
        val dir = context.getExternalFilesDir("settings-shots")!!
        val states = mapOf(
            "no-backup" to SettingsScreenState.Ready("5", ThemeChoice.LIGHT, backupNudge = BackupNudgeShown.None(4200, 812)),
            "last-backup" to SettingsScreenState.Ready("5", ThemeChoice.LIGHT, backupNudge = BackupNudgeShown.Last("3 March", 1240)),
            "report" to SettingsScreenState.Ready(
                "5",
                ThemeChoice.LIGHT,
                backupNudge = BackupNudgeShown.Last("31 October", 0),
                backup = BackupRow.Written("Cairn backup 2026-10-31.ccoll", 812, 4200, BenchProvider.uri("Cairn backup 2026-10-31.ccoll")),
            ),
            "unrecorded" to SettingsScreenState.Ready(
                "5",
                ThemeChoice.LIGHT,
                backupNudge = BackupNudgeShown.Last("3 March", 1240),
                backup = BackupRow.Written(
                    "Cairn backup 2026-10-31.ccoll",
                    812,
                    4200,
                    BenchProvider.uri("Cairn backup 2026-10-31.ccoll"),
                    unrecorded = "disk full",
                ),
            ),
            "failed" to SettingsScreenState.Ready(
                "5",
                ThemeChoice.LIGHT,
                backupNudge = BackupNudgeShown.Last("3 March", 1240),
                backup = BackupRow.Failed("refused"),
            ),
        )
        var dark by mutableStateOf(false)
        var shown by mutableStateOf(states.getValue("no-backup"))
        rule.setContent { CairnTheme(dark) { SettingsPage(shown, {}, {}, {}, {}, {}) } }
        for ((name, state) in states) {
            for (isDark in listOf(false, true)) {
                dark = isDark
                shown = state
                rule.waitForIdle()
                // The row's last line in view: what the report says under it, the report, or the nudge.
                rule.onNodeWithTag(listOf("backup-unrecorded", "backup-share").firstOrNull(::exists) ?: "backup-nudge").performScrollTo()
                rule.waitForIdle()
                val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
                FileOutputStream(File(dir, "backup-$name-${if (isDark) "dark" else "light"}.png")).use {
                    bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
                }
                assertEquals(pageArgb(isDark), bitmap.getPixel(2, bitmap.height / 2))
            }
        }
        shown = states.getValue("report")
        rule.waitForIdle()
        assertEquals("Cairn backup 2026-10-31.ccoll — 812 notes, 4,200 reviews", text("backup-report"))
        assertEquals("Share", text("backup-share"))
        // Share sits beside the report: after it along the line, and level with it.
        val report = rule.onNodeWithTag("backup-report").fetchSemanticsNode().boundsInRoot
        val share = rule.onNodeWithTag("backup-share").fetchSemanticsNode().boundsInRoot
        assertTrue("$share after $report", share.left >= report.right)
        assertTrue("$share level with $report", share.top < report.bottom && report.top < share.bottom)
        shown = states.getValue("unrecorded")
        rule.waitForIdle()
        assertEquals("Saved, but not recorded as the last backup: disk full", text("backup-unrecorded"))
        shown = states.getValue("failed")
        rule.waitForIdle()
        assertEquals("Could not write the backup: refused", text("backup-failed"))
    }
}

/** The manifest of a `.ccoll` archive's bytes. */
internal fun manifestOf(bytes: ByteArray): JSONObject {
    ZipInputStream(bytes.inputStream()).use { zip ->
        while (true) {
            val entry = zip.nextEntry ?: error("no manifest.json in the archive")
            if (entry.name == "manifest.json") return JSONObject(zip.readBytes().decodeToString())
        }
    }
}

/** An ISO-8601 UTC instant with milliseconds, as the manifest's `created` is written, in epoch milliseconds. */
internal fun isoMillis(text: String): Long =
    SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.ENGLISH).apply { timeZone = TimeZone.getTimeZone("UTC") }.parse(text)!!.time
