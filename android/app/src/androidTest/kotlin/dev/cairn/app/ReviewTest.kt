package dev.cairn.app

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.Grade
import dev.cairn.ffi.ReviewStateKind
import dev.cairn.ffi.RunStyle
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/**
 * Calls each exported Review operation once, on a real device, against fixtures installed by name.
 * The rules are pinned by the Rust tests; this proves they cross the binding.
 */
@RunWith(AndroidJUnit4::class)
class ReviewTest {
    private val context: Context = ApplicationProvider.getApplicationContext()
    private val now = 1_760_000_000_000L

    private fun withFixture(name: String, body: (CollectionHandle) -> Unit) {
        val root = File(context.cacheDir, "review-${System.nanoTime()}")
        try {
            CollectionHandle.open(File(root, "files").absolutePath, File(root, "no_backup").absolutePath).use { handle ->
                handle.installFixture(name, now)
                body(handle)
            }
        } finally {
            root.deleteRecursively()
        }
    }

    @Test
    fun the_review_state_arrives_as_facts() {
        withFixture("caught-up") { assertEquals(ReviewStateKind.CaughtUp, it.reviewState(now).state) }
        withFixture("leeches") { assertEquals(3u, it.reviewState(now).leeches) }
        // `open` seeds a first-launch deck, so these two are only reached by name.
        withFixture("empty") { assertEquals(ReviewStateKind.Empty, it.reviewState(now).state) }
        withFixture("new-only") { assertEquals(ReviewStateKind.NewOnly(2u), it.reviewState(now).state) }
        withFixture("backlog") {
            val facts = it.reviewState(now)
            assertTrue(facts.backlog)
            assertEquals(listOf(5u, 10u, 20u), facts.shorter)
            val state = facts.state as ReviewStateKind.Due
            assertEquals(25u, state.due)
            assertTrue(state.backlog)
        }
    }

    @Test
    fun a_sitting_runs_start_to_end() {
        withFixture("backlog") { handle ->
            handle.startSitting(5u, 1_000uL, now).use { sitting ->
                assertEquals(5u, sitting.chosen())
                assertEquals(0u, sitting.graded())
                assertFalse(sitting.checkpointDue(2_000uL))
                assertTrue(sitting.checkpointDue(1_000uL + 600_000uL))
                sitting.keepGoing()

                val shown = sitting.card(now, 2_000uL)!!
                assertFalse(shown.revealed)
                assertFalse(shown.isNew)
                assertTrue(shown.previews.good >= 1u)

                // Graded before the reveal: refused, crossing as the typed reason and no sentence.
                try {
                    sitting.grade(Grade.GOOD, 2_500uL, now)
                    fail("a grade before the reveal is refused")
                } catch (_: CairnException.NotRevealed) {
                }

                sitting.reveal()
                assertTrue(sitting.card(now, 2_600uL)!!.revealed)
                assertTrue(sitting.grade(Grade.GOOD, 3_000uL, now).recorded)

                assertFalse(sitting.ended(now))
                assertEquals(1u, sitting.graded())
                assertEquals(0u, sitting.leechesCrossed(now))
            }
        }
    }

    @Test
    fun a_lapse_comes_straight_back_and_a_crossing_leech_is_counted() {
        withFixture("crossing") { handle ->
            handle.startSitting(1u, 0uL, now).use { sitting ->
                val first = sitting.card(now, 10uL)!!
                sitting.reveal()
                sitting.grade(Grade.FORGOT, 20uL, now)
                assertTrue(sitting.ended(now))
                assertEquals(1u, sitting.leechesCrossed(now))
            }
        }
        withFixture("crossing") { handle ->
            handle.startSitting(5u, 0uL, now).use { sitting ->
                val first = sitting.card(now, 10uL)!!
                sitting.reveal()
                sitting.grade(Grade.FORGOT, 20uL, now)
                val again = sitting.card(now, 30uL)!!
                assertEquals(first.noteId, again.noteId)
                assertFalse(again.revealed)
            }
        }
    }

    @Test
    fun finish_here_ends_the_sitting() {
        withFixture("backlog") { handle ->
            handle.startSitting(10u, 0uL, now).use { sitting ->
                sitting.card(now, 10uL)!!
                sitting.finishHere()
                assertTrue(sitting.ended(now))
            }
        }
    }

    @Test
    fun a_cloze_card_arrives_masked_and_markup_arrives_as_runs() {
        withFixture("cloze") { handle ->
            handle.startSitting(1u, 0uL, now).use { sitting ->
                val shown = sitting.card(now, 10uL)!!
                assertTrue(shown.isNew)
                assertNotEquals(shown.prompt.joinToString("") { it.text }, shown.answer.joinToString("") { it.text })
                assertTrue(shown.prompt.all { it.style in RunStyle.values() })
            }
        }
    }

    @Test
    fun a_persian_prompt_reads_right_to_left_and_a_french_one_does_not() {
        withFixture("persian") { handle ->
            handle.startSitting(5u, 0uL, now).use { sitting ->
                var persian = false
                var french = false
                repeat(5) {
                    val shown = sitting.card(now, 10uL)!!
                    val arabic = shown.prompt.any { run -> run.text.any { it in '؀'..'ۿ' } }
                    assertEquals(arabic, shown.rtl)
                    if (arabic) persian = true else french = true
                    sitting.reveal()
                    sitting.grade(Grade.GOOD, 20uL, now)
                }
                assertTrue(persian && french)
            }
        }
    }

    @Test
    fun a_reveal_or_grade_with_no_card_on_screen_is_refused_as_no_card_shown() {
        withFixture("backlog") { handle ->
            handle.startSitting(5u, 0uL, now).use { sitting ->
                try {
                    sitting.reveal()
                    fail("a reveal with no card shown is refused")
                } catch (_: CairnException.NoCardShown) {
                }
                try {
                    sitting.grade(Grade.GOOD, 1uL, now)
                    fail("a grade with no card shown is refused")
                } catch (_: CairnException.NoCardShown) {
                }
            }
        }
    }

    @Test
    fun an_unknown_fixture_crosses_as_unknown_fixture_naming_it() {
        val root = File(context.cacheDir, "review-unknown-${System.nanoTime()}")
        try {
            CollectionHandle.open(File(root, "files").absolutePath, File(root, "no_backup").absolutePath).use {
                try {
                    it.installFixture("no-such-fixture", now)
                    fail("an unknown fixture is refused")
                } catch (e: CairnException.UnknownFixture) {
                    assertEquals("no-such-fixture", e.name)
                }
            }
        } finally {
            root.deleteRecursively()
        }
    }
}
