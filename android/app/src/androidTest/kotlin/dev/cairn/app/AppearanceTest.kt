package dev.cairn.app

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.ThemeChoice
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

@RunWith(AndroidJUnit4::class)
class AppearanceTest {
    private val context: Context = ApplicationProvider.getApplicationContext()

    @Test
    fun light_and_dark_are_fixed_and_system_follows_the_platform() {
        for (system in listOf(true, false)) {
            assertEquals(false, isDark(ThemeChoice.LIGHT, system))
            assertEquals(true, isDark(ThemeChoice.DARK, system))
            assertEquals(system, isDark(ThemeChoice.SYSTEM, system))
        }
    }

    @Test
    fun the_stored_choice_is_read_through_the_binding_and_anything_else_follows_the_platform() {
        val root = File(context.cacheDir, "appearance-${System.nanoTime()}")
        val data = File(root, "files")
        val state = File(root, "no_backup")
        try {
            fun read() = CollectionHandle.open(data.absolutePath, state.absolutePath).use { it.themeChoice() }

            assertEquals("unset", ThemeChoice.SYSTEM, read())
            // The setter only writes the three valid forms, so the row is written the way an earlier
            // session of the desktop app would have left it (an unknown value included), with the collection closed.
            for ((stored, expected) in listOf(
                "dark" to ThemeChoice.DARK,
                "light" to ThemeChoice.LIGHT,
                "system" to ThemeChoice.SYSTEM,
                "sepia" to ThemeChoice.SYSTEM,
            )) {
                val db = SQLiteDatabase.openDatabase(File(data, "collection.db").path, null, SQLiteDatabase.OPEN_READWRITE)
                try {
                    db.execSQL("INSERT OR REPLACE INTO local (key, value) VALUES ('theme_preference', ?)", arrayOf(stored))
                } finally {
                    db.close()
                }
                assertEquals("stored $stored", expected, read())
            }
        } finally {
            root.deleteRecursively()
        }
    }
}
