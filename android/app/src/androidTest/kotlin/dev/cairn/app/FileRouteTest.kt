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
 * Who is offered a file by *Open with* (#256): a file manager that types the file as any type at all, or
 * as a zip, or sends it with no type, still reaches the app when the name says it is ours. Resolution only;
 * the bytes still decide what the file is (ADR-0047 §2). Nothing here touches the collection.
 */
@RunWith(AndroidJUnit4::class)
class FileRouteTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private val ours = ComponentName(context, MainActivity::class.java)

    private fun offered(uri: String, type: String?): Boolean {
        val intent = Intent(Intent.ACTION_VIEW).addCategory(Intent.CATEGORY_BROWSABLE).setPackage(context.packageName)
        if (type == null) intent.data = Uri.parse(uri) else intent.setDataAndType(Uri.parse(uri), type)
        return context.packageManager.queryIntentActivities(intent, PackageManager.MATCH_DEFAULT_ONLY).any {
            it.activityInfo.name == ours.className
        }
    }

    @Test
    fun an_archive_is_offered_whatever_type_the_file_manager_gives_it() {
        for (scheme in listOf("content://files.example/doc", "file:///storage/emulated/0/Download")) {
            for (type in listOf("application/octet-stream", "*/*", "application/zip", "application/x-zip-compressed", null)) {
                assertTrue("$scheme, $type", offered("$scheme/Cairn backup 2026-09-01.ccoll", type))
                assertTrue("$scheme, $type, one dot", offered("$scheme/backup.ccoll", type))
                assertTrue("$scheme, $type, a deck file", offered("$scheme/French A1.cdeck", type))
            }
        }
    }

    @Test
    fun a_file_that_is_not_ours_by_name_is_still_offered_only_as_before() {
        for (type in listOf("application/zip", null)) {
            assertFalse("$type", offered("content://files.example/doc/holiday.zip", type))
        }
        assertTrue(offered("content://files.example/doc/holiday.zip", "application/octet-stream"))
    }
}
