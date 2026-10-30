package dev.cairn.app

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.InternalException
import android.os.Looper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

@RunWith(AndroidJUnit4::class)
class CollectionTest {
    private val context: Context = ApplicationProvider.getApplicationContext()

    @Test
    fun the_owner_opens_the_collection_under_files_and_no_backup() {
        runBlocking { CollectionOwner(context).use { it.noteCount() } }

        assertTrue(File(context.filesDir, "collection.db").exists())
        assertTrue(File(context.noBackupFilesDir, "derived.db").exists())
        assertTrue("the writer marker", File(context.noBackupFilesDir, "writer.id").exists())
        assertTrue("nothing of the store belongs in the backed-up directory", !File(context.filesDir, "derived.db").exists())
    }

    @Test
    fun a_call_made_from_the_main_thread_runs_off_it() {
        // StrictMode sees Java-level I/O, not the native SQLite inside the binding, so the thread is
        // checked directly: the owner moves every call to IO whatever thread asked.
        val ranOnMain = runBlocking(Dispatchers.Main) {
            check(Looper.myLooper() == Looper.getMainLooper())
            CollectionOwner(context).use { Looper.myLooper() == Looper.getMainLooper() }
        }
        assertEquals(false, ranOnMain)
    }

    @Test
    fun a_first_launch_seeds_and_a_second_open_does_not_seed_again() {
        val root = File(context.cacheDir, "first-launch-${System.nanoTime()}")
        val data = File(root, "files").absolutePath
        val state = File(root, "no_backup").absolutePath
        try {
            val seeded = CollectionHandle.open(data, state).use { it.noteCount() }
            assertTrue("a first launch seeds the starter notes", seeded > 0u)
            val again = CollectionHandle.open(data, state).use { it.noteCount() }
            assertEquals(seeded, again)
        } finally {
            root.deleteRecursively()
        }
    }

    @Test
    fun an_open_that_fails_crosses_as_cairn_exception() {
        val file = File(context.cacheDir, "not-a-directory-${System.nanoTime()}").apply { writeText("x") }
        try {
            CollectionHandle.open(File(file, "d").absolutePath, File(file, "s").absolutePath).close()
            fail("opening under a file must fail")
        } catch (_: CairnException) {
        } finally {
            file.delete()
        }
    }

    @Test
    fun a_rust_panic_reaches_kotlin_as_an_exception_and_the_handle_survives() {
        val root = File(context.cacheDir, "panic-${System.nanoTime()}")
        try {
            CollectionHandle.open(File(root, "files").absolutePath, File(root, "no_backup").absolutePath).use { handle ->
                try {
                    handle.panicProbe()
                    fail("the probe panics")
                } catch (e: InternalException) {
                    assertTrue(e.message!!.contains("panic probe"))
                }
                assertEquals(0uL, handle.dueCount(System.currentTimeMillis()))
            }
        } finally {
            root.deleteRecursively()
        }
    }
}
