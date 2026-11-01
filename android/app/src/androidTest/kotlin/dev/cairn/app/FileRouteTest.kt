package dev.cairn.app

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Who is offered a file by *Open with* (#256). Files by Google's browse-by-folder route asks the system
 * who opens a `.ccoll` by a type that is neither of the two the app names, and shows only what matches.
 * Resolution only: the bytes still decide what the file is (ADR-0024 §1), and nothing here touches the
 * collection.
 */
@RunWith(AndroidJUnit4::class)
class FileRouteTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private val ours = ComponentName(context, MainActivity::class.java)

    private fun offered(uri: String, type: String): Boolean {
        val intent = Intent(Intent.ACTION_VIEW).addCategory(Intent.CATEGORY_BROWSABLE).setPackage(context.packageName)
        intent.setDataAndType(Uri.parse(uri), type)
        return context.packageManager.queryIntentActivities(intent, PackageManager.MATCH_DEFAULT_ONLY).any {
            it.activityInfo.name == ours.className
        }
    }

    @Test
    fun an_archive_is_offered_whatever_type_the_file_manager_asks_by() {
        val file = "content://com.google.android.apps.nbu.files.provider/1/file:///storage/emulated/0/Download/Cairn%20backup.ccoll"
        for (type in listOf("*/*", "application/zip", "application/x-zip-compressed", "application/octet-stream")) {
            assertTrue("$file, $type", offered(file, type))
        }
    }

    @Test
    fun the_any_type_door_opens_for_that_file_manager_alone() {
        val elsewhere = listOf(
            "content://com.example.other.provider/1/Cairn%20backup.ccoll",
            "file:///storage/emulated/0/Download/Cairn%20backup.ccoll",
        )
        for (uri in elsewhere) {
            assertFalse(uri, offered(uri, "application/zip"))
            assertTrue(uri, offered(uri, "application/octet-stream"))
        }
    }
}
