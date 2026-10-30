package dev.cairn.app

import android.os.SystemClock
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.ReviewFacts
import dev.cairn.ffi.SittingHandle
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** What Review shows: the entrance as facts, or a sitting that has been begun. */
sealed interface ReviewScreenState {
    data object Loading : ReviewScreenState

    data class Entrance(val facts: ReviewFacts) : ReviewScreenState

    /** A sitting of [chosen] gradings. The sitting itself is drawn by the next ticket. */
    class Sitting(val handle: SittingHandle, val chosen: Int) : ReviewScreenState

    data object Failed : ReviewScreenState
}

/**
 * Review's state. Every call into the binding goes through the owner, which moves it to
 * `Dispatchers.IO`; this holds the facts and the sitting handle (ADR-0046 §7) and decides nothing a
 * state does not say. Time is handed over as values: wall-clock milliseconds for the day and the
 * review rows, a monotonic count for durations.
 */
class ReviewViewModel(private val owner: CollectionOwner) : ViewModel() {
    private val _state = MutableStateFlow<ReviewScreenState>(ReviewScreenState.Loading)
    val state: StateFlow<ReviewScreenState> = _state

    init {
        viewModelScope.launch { load() }
    }

    private suspend fun load() {
        _state.value = try {
            ReviewScreenState.Entrance(owner.use { it.reviewState(System.currentTimeMillis()) })
        } catch (_: CairnException) {
            ReviewScreenState.Failed
        }
    }

    /** Begin a sitting of [count] gradings. */
    fun start(count: Int) {
        viewModelScope.launch {
            _state.value = try {
                val handle = owner.use {
                    it.startSitting(count.toUInt(), SystemClock.elapsedRealtime().toULong(), System.currentTimeMillis())
                }
                ReviewScreenState.Sitting(handle, count)
            } catch (_: CairnException) {
                ReviewScreenState.Failed
            }
        }
    }
}
