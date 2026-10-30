package dev.cairn.app

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.Grade
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/**
 * What the shipping configuration must still do once R8 has renamed and stripped the app. Run it
 * against the minified build with `-Pcairn.testRelease`; it uses only the binding, which the keep
 * rules leave under its own names, so the test APK links against the minified one.
 */
@RunWith(AndroidJUnit4::class)
class ReleaseBuildTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private val now = 1_760_000_000_000L

    private fun withCollection(body: (CollectionHandle) -> Unit) {
        val root = File(context.cacheDir, "release-${System.nanoTime()}")
        try {
            CollectionHandle.open(File(root, "files").absolutePath, File(root, "no_backup").absolutePath).use { handle ->
                assertTrue(File(root, "files/collection.db").exists())
                assertTrue(File(root, "no_backup/derived.db").exists())
                body(handle)
            }
        } finally {
            root.deleteRecursively()
        }
    }

    @Test
    fun a_sitting_runs_to_its_end() {
        withCollection { handle ->
            handle.installFixture("backlog", now)
            handle.startSitting(5u, 0uL, now).use { sitting ->
                var t = 1_000uL
                while (!sitting.ended(now)) {
                    assertTrue(sitting.card(now, t) != null)
                    sitting.reveal()
                    assertTrue(sitting.grade(Grade.GOOD, t + 500uL, now).recorded)
                    t += 1_000uL
                }
                assertEquals(5u, sitting.graded())
            }
        }
    }

    @Test
    fun a_rust_error_reaches_kotlin_as_cairn_exception() {
        withCollection { handle ->
            try {
                handle.installFixture("no-such-fixture", now)
                fail("an unknown fixture must be refused")
            } catch (e: CairnException) {
                assertTrue(e::class.java.name.startsWith("dev.cairn.ffi.CairnException"))
            }
        }
    }
}
