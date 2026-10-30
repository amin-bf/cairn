package dev.cairn.app

import android.os.SystemClock
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.Grade
import dev.cairn.ffi.ReviewFacts
import dev.cairn.ffi.ShownCard
import dev.cairn.ffi.SittingHandle
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** What Review shows: the entrance as facts, or a sitting that has been begun. */
sealed interface ReviewScreenState {
    data object Loading : ReviewScreenState

    data class Entrance(val facts: ReviewFacts) : ReviewScreenState

    /**
     * A sitting under way: the gradings made against the count chosen, the card shown, and whether
     * the ten-minute check-in is waiting above it. All three are facts the sitting reported.
     */
    data class Sitting(val chosen: Int, val graded: Int, val card: ShownCard, val checkpoint: Boolean) : ReviewScreenState

    data object Failed : ReviewScreenState
}

/**
 * Review's state. Every call into the binding goes through the owner, which moves it to
 * `Dispatchers.IO`; this holds the facts and the sitting handle (ADR-0046 §7) and decides nothing a
 * state does not say — not whether a grade is allowed (the sitting refuses it), not when the
 * checkpoint falls due, not whether the sitting is over. Time is handed over as values: wall-clock
 * milliseconds for the day and the review rows, a monotonic count for durations; a test hands its own.
 */
class ReviewViewModel(
    private val owner: CollectionOwner,
    private val wallClock: () -> Long = System::currentTimeMillis,
    private val monotonicClock: () -> Long = SystemClock::elapsedRealtime,
) : ViewModel() {
    private val _state = MutableStateFlow<ReviewScreenState>(ReviewScreenState.Loading)
    val state: StateFlow<ReviewScreenState> = _state

    // Held only in memory: a process death drops the sitting (ADR-0006 §6).
    private var sitting: SittingHandle? = null

    init {
        viewModelScope.launch { load() }
    }

    private suspend fun load() {
        sitting = null
        _state.value = try {
            ReviewScreenState.Entrance(owner.use { it.reviewState(wallClock()) })
        } catch (_: CairnException) {
            ReviewScreenState.Failed
        }
    }

    /** Begin a sitting of [count] gradings. */
    fun start(count: Int) {
        viewModelScope.launch {
            try {
                sitting = owner.use { it.startSitting(count.toUInt(), monotonicClock().toULong(), wallClock()) }
                show()
            } catch (_: CairnException) {
                _state.value = ReviewScreenState.Failed
            }
        }
    }

    /** Re-ask the sitting what is on screen. A sitting that has ended returns straight to the entrance. */
    private suspend fun show() {
        val handle = sitting ?: return
        try {
            val now = wallClock()
            val mono = monotonicClock().toULong()
            // One trip off the main thread: every call on the handle blocks the thread it is made on.
            val shown = owner.use {
                val card = if (handle.ended(now)) null else handle.card(now, mono)
                card?.let {
                    ReviewScreenState.Sitting(
                        chosen = handle.chosen().toInt(),
                        graded = handle.graded().toInt(),
                        card = it,
                        checkpoint = handle.checkpointDue(mono),
                    )
                }
            }
            if (shown == null) load() else _state.value = shown
        } catch (_: CairnException) {
            _state.value = ReviewScreenState.Failed
        }
    }

    /** The card was tapped. The sitting has one cause for a reveal; tapping a revealed card does nothing. */
    fun reveal() {
        val shown = (_state.value as? ReviewScreenState.Sitting)?.card ?: return
        if (shown.revealed) return
        viewModelScope.launch {
            try {
                owner.use { sitting?.reveal() }
                show()
            } catch (_: CairnException) {
                _state.value = ReviewScreenState.Failed
            }
        }
    }

    /** A grade was pressed. The sitting refuses one before the reveal; a refusal changes nothing here. */
    fun grade(grade: Grade) {
        viewModelScope.launch {
            try {
                owner.use { sitting?.grade(grade, monotonicClock().toULong(), wallClock()) }
            } catch (_: CairnException) {
                // Refused: nothing was recorded and nothing is counted. The card stays as it was.
            }
            show()
        }
    }

    /** *Keep going*: the check-in does not come back this sitting. */
    fun keepGoing() {
        viewModelScope.launch {
            owner.use { sitting?.keepGoing() }
            show()
        }
    }

    /** *Finish here*: the sitting ends and the entrance is shown. */
    fun finishHere() {
        viewModelScope.launch {
            owner.use { sitting?.finishHere() }
            show()
        }
    }

    /** Time has passed with no input: ask again, so the check-in can fall due while a card is looked at. */
    fun tick() {
        if (_state.value is ReviewScreenState.Sitting) viewModelScope.launch { show() }
    }
}
