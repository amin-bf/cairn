package dev.leitner.fsrsbench;

import android.app.Activity;
import android.os.Bundle;
import android.util.Log;
import android.widget.TextView;

/**
 * Runs the FSRS optimiser inside a real app process. The shell runs `adb shell` binaries
 * in a different cgroup from an app, so the shell measurement alone does not answer what
 * the app will experience.
 *
 * <p>`delay_ms` exists so the run can be started, then the app pushed to the background
 * with HOME before the work begins — that measures the throttled case.
 */
public class MainActivity extends Activity {

    static {
        System.loadLibrary("fsrsbench");
    }

    /** Returns the report text. Implemented in Rust; see core/src/android.rs. */
    public static native String runBench(String spec);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        TextView view = new TextView(this);
        view.setText("fsrsbench running — see logcat tag FSRSBENCH");
        setContentView(view);

        final String spec = getIntent().getStringExtra("spec") != null
                ? getIntent().getStringExtra("spec")
                : "5000,20000,73000,250000,730000";
        final long delayMs = getIntent().getLongExtra("delay_ms", 0L);
        final String tag = getIntent().getStringExtra("tag") != null
                ? getIntent().getStringExtra("tag")
                : "foreground";

        new Thread(() -> {
            try {
                if (delayMs > 0) {
                    Thread.sleep(delayMs);
                }
            } catch (InterruptedException ignored) {
                // Nothing to do; fall through and run anyway.
            }
            Log.i("FSRSBENCH", "RUN_BEGIN where=" + tag + " spec=" + spec);
            String report = runBench(spec);
            // logcat truncates long entries, so emit one line per entry.
            for (String line : report.split("\n")) {
                Log.i("FSRSBENCH", line);
            }
            Log.i("FSRSBENCH", "RUN_END where=" + tag);
        }).start();
    }
}
