package dev.cairn.app

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.InternalException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/** Debug builds only: the `panic-probe` export a release build never carries (see app/build.gradle.kts). */
@RunWith(AndroidJUnit4::class)
class PanicProbeTest {
    private val context: Context = ApplicationProvider.getApplicationContext()

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
