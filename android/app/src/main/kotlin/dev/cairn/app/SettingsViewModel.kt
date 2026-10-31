package dev.cairn.app

import android.net.Uri
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.BackupNudge
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.OptimisationNudge
import dev.cairn.ffi.OptimiseFinish
import dev.cairn.ffi.OptimiseHandle
import dev.cairn.ffi.OptimisePhase
import dev.cairn.ffi.ThemeChoice
import dev.cairn.ffi.backupFilename
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/** The fact under the Scheduler label (ADR-0014 §2): counts only; the sentence is the string resources'. */
sealed interface Nudge {
    data class Standard(val reviewsTotal: Long) : Nudge

    data class Fitted(val fittedOver: Long, val reviewsSince: Long) : Nudge
}

/** Where an Optimise run is, as the page draws it (ADR-0014 §4). */
sealed interface OptimiseRun {
    data object Idle : OptimiseRun

    /** The uncancellable lead-in: an indeterminate indicator. */
    data object Preparing : OptimiseRun

    data class Training(val current: Long, val total: Long) : OptimiseRun
}

/** The fact under *Back up…* (ADR-0016 §6): counts and a date; the sentence is the string resources'. */
sealed interface BackupNudgeShown {
    data class None(val reviews: Long, val notes: Long) : BackupNudgeShown

    /** [date] is the day the last backup was made, as [backupDate] words it. */
    data class Last(val date: String, val reviewsSince: Long) : BackupNudgeShown
}

/** What the backup row says beside its nudge (ADR-0053 §4). */
sealed interface BackupRow {
    data object Idle : BackupRow

    data object Running : BackupRow

    /** Written: [name] is the document as the provider named it, or null when it gave none. */
    data class Written(val name: String?, val notes: Long, val reviews: Long) : BackupRow

    data class Failed(val error: String) : BackupRow
}

/**
 * The day an instant falls on in [zone]: *3 March*, with the year when it is not [nowMs]'s year
 * (*3 March 2025*). English only, as every sentence round it is.
 */
fun backupDate(atMs: Long, nowMs: Long, zone: TimeZone = TimeZone.getDefault()): String {
    val year = { ms: Long -> Calendar.getInstance(zone).apply { timeInMillis = ms }.get(Calendar.YEAR) }
    val pattern = if (year(atMs) == year(nowMs)) "d MMMM" else "d MMMM yyyy"
    return SimpleDateFormat(pattern, Locale.ENGLISH).apply { timeZone = zone }.format(Date(atMs))
}

/** How far east of UTC the device's zone is at [ms], in minutes: what dates a backup's name. */
private fun offsetMinutes(ms: Long): Int = TimeZone.getDefault().getOffset(ms) / 60_000

/** The SavedStateHandle key of a backup's creation instant, kept while the save dialog is up. */
private const val BACKUP_CREATED = "backup-created-ms"

/** No documents to write: for a model built with no save dialog behind it. */
private object NoDocuments : DocumentWriter {
    override suspend fun write(uri: Uri, bytes: ByteArray): String? = throw IOException("no document provider")

    override suspend fun delete(uri: Uri) {}
}

/**
 * What Settings shows: the rate field as typed, the stored appearance, the block that last refused a
 * write, the Scheduler's nudge, the Optimise run, whether a run has just written (the completion
 * sentence stays until the next run starts), and the backup's nudge and row.
 */
sealed interface SettingsScreenState {
    data object Loading : SettingsScreenState

    data class Ready(
        val rate: String,
        val choice: ThemeChoice,
        val rateRefused: Boolean = false,
        val appearanceRefused: Boolean = false,
        val nudge: Nudge? = null,
        val run: OptimiseRun = OptimiseRun.Idle,
        val updated: Boolean = false,
        val backupNudge: BackupNudgeShown? = null,
        val backup: BackupRow = BackupRow.Idle,
        /** The collection's size in whole MB once it is past the automatic backup's quota; null under it. */
        val backupOverQuotaMb: Long? = null,
    ) : SettingsScreenState

    data object Failed : SettingsScreenState
}

private fun BackupNudge.shown(nowMs: Long): BackupNudgeShown = when (val at = lastBackupMs) {
    null -> BackupNudgeShown.None(reviews.toLong(), notes.toLong())
    else -> BackupNudgeShown.Last(backupDate(at, nowMs), reviewsSince.toLong())
}

private fun OptimisationNudge.toNudge(): Nudge = when (this) {
    is OptimisationNudge.Standard -> Nudge.Standard(reviewsTotal.toLong())
    is OptimisationNudge.Fitted -> Nudge.Fitted(fittedOver.toLong(), reviewsSince.toLong())
}

/**
 * Settings' state (ADR-0042, ADR-0051). The rate's clamp, the stored forms and what a stored theme
 * means are the binding's (ADR-0046 §1); this holds what was typed, commits it when the person
 * finishes, and reads the block again after a refusal. A write to Appearance updates [appearance], so
 * the whole app changes theme at once.
 *
 * It also holds the Optimise run (ADR-0014, client-stack rule 10): the handle lives here, so a run outlives a
 * rotation and a switch of destination. [tick] polls it; the screen calls that on a short interval
 * while its lifecycle is started. No service, no wakelock, no scheduled work: a run that nobody
 * watches is simply not seen to finish until the screen is back.
 *
 * And it holds a backup (ADR-0053 §4): the creation instant is taken once, when the save dialog opens,
 * and kept in [saved], so a rotation or a killed process behind the dialog still names and dates the
 * archive by it. Only a write the provider took is recorded.
 */
class SettingsViewModel(
    private val owner: CollectionAccess,
    private val appearance: AppearanceState,
    private val nowWallMs: () -> Long = System::currentTimeMillis,
    private val saved: SavedStateHandle = SavedStateHandle(),
    private val documents: DocumentWriter = NoDocuments,
) : ViewModel() {
    private val _state = MutableStateFlow<SettingsScreenState>(SettingsScreenState.Loading)
    val state: StateFlow<SettingsScreenState> = _state

    /** Whether the field holds something the person typed that was not yet committed; a read leaves it alone. */
    private var editing = false

    /** The run in flight, if any. Touched from the main thread and from [tick]'s coroutine, one at a time. */
    private var run: OptimiseHandle? = null

    /** Whether a run is being started, so a second request before the first has a handle starts nothing. */
    private var starting = false

    /** Whether a [tick] is under way, so a slow poll is not overtaken by the next one. */
    private var ticking = false

    init {
        refresh()
    }

    /** Read the page again; a field the person is mid-way through is not overwritten, nor is a run. */
    fun refresh() {
        viewModelScope.launch { read(rateRefused = false, appearanceRefused = false) }
    }

    private suspend fun read(rateRefused: Boolean, appearanceRefused: Boolean, rereadRate: Boolean = false) {
        try {
            val (rate, choice, nudge) = owner.use { Triple(it.newCardRate(), it.themeChoice(), it.optimisationNudge().toNudge()) }
            val backupFacts = owner.use { it.backupNudge() }
            val backupNudge = backupFacts.shown(nowWallMs())
            val before = _state.value as? SettingsScreenState.Ready
            val shown = before?.rate
            val keep = editing && !rereadRate && shown != null
            if (rereadRate) editing = false
            _state.value = SettingsScreenState.Ready(
                rate = if (keep) shown!! else rate.toString(),
                choice = choice,
                rateRefused = rateRefused,
                appearanceRefused = appearanceRefused,
                nudge = nudge,
                run = before?.run ?: OptimiseRun.Idle,
                updated = before?.updated ?: false,
                backupNudge = backupNudge,
                backup = before?.backup ?: BackupRow.Idle,
                backupOverQuotaMb = backupFacts.sizeMb.toLong().takeIf { backupFacts.overQuota },
            )
        } catch (_: CairnException) {
            _state.value = SettingsScreenState.Failed
        }
    }

    /** The field changed: what is typed is kept as typed. */
    fun editRate(text: String) {
        val ready = _state.value as? SettingsScreenState.Ready ?: return
        editing = true
        _state.value = ready.copy(rate = text, rateRefused = false)
    }

    /** *Done*, or the field losing focus: commit what is typed if it parses, and show the value stored. */
    fun commitRate() {
        val ready = _state.value as? SettingsScreenState.Ready ?: return
        val typed = ready.rate.trim()
        // Digits too many for the binding's integer are still a number above the limit: the clamp answers it.
        val rate = typed.toUIntOrNull() ?: if (typed.isNotEmpty() && typed.all { it in '0'..'9' }) UInt.MAX_VALUE else return
        viewModelScope.launch {
            try {
                val stored = owner.use { it.setNewCardRate(rate) }
                editing = false
                _state.value = (_state.value as? SettingsScreenState.Ready)?.copy(rate = stored.toString(), rateRefused = false)
                    ?: return@launch
            } catch (_: CairnException) {
                read(rateRefused = true, appearanceRefused = false, rereadRate = true)
            }
        }
    }

    /** A choice of appearance: written, then applied to the whole app. */
    fun chooseAppearance(choice: ThemeChoice) {
        viewModelScope.launch {
            try {
                owner.use { it.setThemeChoice(choice) }
                appearance.choice = choice
                (_state.value as? SettingsScreenState.Ready)?.let {
                    _state.value = it.copy(choice = choice, appearanceRefused = false)
                }
            } catch (_: CairnException) {
                read(rateRefused = false, appearanceRefused = true)
            }
        }
    }

    /** *Optimise*: start a run from the foreground (client-stack rule 10). The completion sentence clears. */
    fun optimise() {
        if (run != null || starting) return
        starting = true
        viewModelScope.launch {
            try {
                run = owner.use { it.startOptimise() }
                (_state.value as? SettingsScreenState.Ready)?.let {
                    _state.value = it.copy(run = OptimiseRun.Preparing, updated = false)
                }
            } catch (_: CairnException) {
                // The run could not start: the page stays at rest, as it was.
            } finally {
                starting = false
            }
        }
    }

    /** *Cancel*: honoured once training is under way; the next poll finds the run over and nothing written. */
    fun cancelOptimise() {
        val handle = run ?: return
        viewModelScope.launch {
            owner.use { handle.cancel() }
            tick()
        }
    }

    /**
     * One poll of the run: it finishes (and a written vector is read back into the nudge), or it shows
     * where it is. A poll that fails leaves the run to be polled again.
     */
    suspend fun tick() {
        val handle = run ?: return
        if (ticking) return
        ticking = true
        try {
            val (finish, phase) = owner.use { handle.finish(nowWallMs()) to handle.phase() }
            when (finish) {
                OptimiseFinish.STILL_RUNNING -> setRun(
                    when (phase) {
                        is OptimisePhase.Training -> OptimiseRun.Training(phase.current.toLong(), phase.total.toLong())
                        else -> OptimiseRun.Preparing
                    },
                )
                OptimiseFinish.WRITTEN -> {
                    run = null
                    setRun(OptimiseRun.Idle, updated = true)
                    read(rateRefused = false, appearanceRefused = false)
                }
                OptimiseFinish.NOTHING_WRITTEN -> {
                    run = null
                    setRun(OptimiseRun.Idle)
                }
            }
        } catch (_: CairnException) {
            // Polled again on the next tick.
        } finally {
            ticking = false
        }
    }

    /**
     * *Back up…*: take the creation instant once and keep it, and return the dated name the save dialog
     * opens with. Nothing else changes until a document is chosen. The name is the binding's, so it is
     * asked for off the main thread.
     */
    suspend fun startBackup(): String {
        val created = nowWallMs()
        saved[BACKUP_CREATED] = created
        return withContext(Dispatchers.IO) { backupFilename(created, offsetMinutes(created)) }
    }

    /** The dialog closed with nothing chosen: nothing is written and nothing is said. */
    fun backupDismissed() {
        saved.remove<Long>(BACKUP_CREATED)
    }

    /**
     * A document was chosen: build the archive at the kept instant, write it, and only then record it
     * and read the nudge again. A failed build or write is said in the row, nothing is recorded, and
     * the half-written document is deleted if the provider allows it.
     */
    fun backUp(uri: Uri) {
        if ((_state.value as? SettingsScreenState.Ready)?.backup == BackupRow.Running) return
        // Taken once, as the dialog opened, and kept across a turn and a process death. Only a document
        // with no dialog behind it, which the launcher never hands back, falls to now: there is no
        // suggested name for the archive to disagree with then.
        val created = saved.get<Long>(BACKUP_CREATED) ?: nowWallMs()
        saved.remove<Long>(BACKUP_CREATED)
        setBackup(BackupRow.Running)
        viewModelScope.launch {
            var written = false
            val row = try {
                val built = owner.use { it.backup(created, offsetMinutes(created)) }
                // The provider's name or none: the suggestion may not be what the document got (ADR-0047 §4).
                val name = documents.write(uri, built.bytes)
                written = true
                owner.use { it.recordBackup(created) }
                BackupRow.Written(name, built.notes.toLong(), built.reviews.toLong())
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // A whole archive is kept though recording it failed; only a half-written one goes.
                if (!written) documents.delete(uri)
                BackupRow.Failed(e.message ?: e.javaClass.simpleName)
            }
            setBackup(row)
            read(rateRefused = false, appearanceRefused = false)
        }
    }

    /** Settings was left: the row's report is not kept for the next visit. A backup in flight still reports. */
    fun leaveBackup() {
        val ready = _state.value as? SettingsScreenState.Ready ?: return
        if (ready.backup != BackupRow.Running) setBackup(BackupRow.Idle)
    }

    private fun setBackup(row: BackupRow) {
        (_state.value as? SettingsScreenState.Ready)?.let { _state.value = it.copy(backup = row) }
    }

    private fun setRun(run: OptimiseRun, updated: Boolean? = null) {
        (_state.value as? SettingsScreenState.Ready)?.let {
            _state.value = it.copy(run = run, updated = updated ?: it.updated)
        }
    }
}
