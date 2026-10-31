package dev.cairn.app

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The one read of a document (#242, ADR-0052): whole under the ceiling, its head past it, and nothing on a failure. */
@RunWith(AndroidJUnit4::class)
class DocumentReaderTest {
    private val resolver = ApplicationProvider.getApplicationContext<Context>().contentResolver

    @After
    fun tearDown() = BenchProvider.files.clear()

    @Test
    fun a_document_under_the_ceiling_is_read_whole_with_its_display_name() {
        BenchProvider.files["a.cdeck"] = ByteArray(100) { it.toByte() }
        val read = runBlocking { readDocument(resolver, BenchProvider.uri("a.cdeck"), ceiling = 100) }
        assertEquals("a.cdeck", read.name)
        assertTrue(read.complete)
        assertArrayEquals(ByteArray(100) { it.toByte() }, read.bytes)
    }

    @Test
    fun a_document_past_the_ceiling_is_cut_to_its_head_and_marked_incomplete() {
        BenchProvider.files["big"] = ByteArray(101) { it.toByte() }
        val read = runBlocking { readDocument(resolver, BenchProvider.uri("big"), ceiling = 100) }
        assertFalse(read.complete)
        assertArrayEquals(ByteArray(100) { it.toByte() }, read.bytes)
    }

    @Test
    fun the_ceiling_is_thirty_two_mebibytes() {
        assertEquals(32 * 1024 * 1024, READ_CEILING_BYTES)
    }

    @Test
    fun a_provider_with_no_name_gives_none() {
        BenchProvider.files["x"] = ByteArray(3)
        val read = runBlocking { readDocument(resolver, BenchProvider.uri("x", "nameless")) }
        assertNull(read.name)
        assertTrue(read.complete)
    }

    @Test
    fun a_failed_read_is_an_empty_incomplete_read() {
        BenchProvider.files["x"] = ByteArray(3)
        val read = runBlocking { readDocument(resolver, BenchProvider.uri("x", "fails")) }
        assertFalse(read.complete)
        assertEquals(0, read.bytes.size)
    }
}
