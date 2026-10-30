package dev.cairn.app

import android.app.Application
import android.content.pm.ApplicationInfo
import android.os.Build
import android.os.StrictMode
import android.util.Log
import java.util.concurrent.CopyOnWriteArrayList

class CairnApplication : Application() {
    /** The process's one collection (ADR-0046 §7). */
    val collection: CollectionOwner by lazy { CollectionOwner(this) }

    /**
     * StrictMode violations whose stack passes through our own code or the binding, in debug builds.
     * The framework trips StrictMode on its own startup reads, which are not ours to fix, so only
     * violations that name `dev.cairn` frames are kept — and an instrumented test asserts there are none.
     */
    val strictModeViolations = CopyOnWriteArrayList<String>()

    override fun onCreate() {
        super.onCreate()
        if (applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0) {
            enableStrictMode()
        }
    }

    // Any call into the binding that runs on the main thread is a bug: it blocks the thread it is made
    // on (ADR-0046 §7). The listener needs API 28; below it the log line is all StrictMode gives, and
    // CollectionOwner hops to Dispatchers.IO itself (a test pins that), so a violation here is new code
    // that bypassed it. StrictMode sees Java-level I/O only, not native SQLite.
    private fun enableStrictMode() {
        val thread = StrictMode.ThreadPolicy.Builder().detectAll().penaltyLog()
        val vm = StrictMode.VmPolicy.Builder().detectAll().penaltyLog()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            val record = StrictMode.OnThreadViolationListener { violation -> note(violation) }
            thread.penaltyListener(Runnable::run, record)
            vm.penaltyListener(Runnable::run, StrictMode.OnVmViolationListener { note(it) })
        }
        StrictMode.setThreadPolicy(thread.build())
        StrictMode.setVmPolicy(vm.build())
    }

    private fun note(violation: android.os.strictmode.Violation) {
        val trace = Log.getStackTraceString(violation)
        if ("dev.cairn" in trace) strictModeViolations += trace
    }
}
