import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.FileCollection
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import javax.inject.Inject

/**
 * What the two Rust tasks share: the workspace they run in, the cargo features of the variant, and the
 * Rust sources whose change makes them stale.
 */
abstract class RustTask : DefaultTask() {
    @get:Inject
    abstract val exec: ExecOperations

    /** The repository root, where the Cargo workspace is. */
    @get:Internal
    abstract val workspace: DirectoryProperty

    /** Cargo features of `cairn-ffi` the variant builds with (`panic-probe` on debug). */
    @get:Input
    abstract val features: ListProperty<String>

    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    val rustSources: FileCollection
        get() = project.fileTree(workspace) {
            include("Cargo.toml", "Cargo.lock", "crates/**")
            exclude("**/target/**")
        }

    protected fun cargo(args: List<String>, env: Map<String, String> = emptyMap()) {
        exec.exec {
            workingDir = workspace.get().asFile
            environment(env)
            commandLine(listOf("cargo") + args)
        }
    }

    protected fun featureArgs(extra: List<String> = emptyList()): List<String> {
        val all = features.get() + extra
        return if (all.isEmpty()) emptyList() else listOf("--features", all.joinToString(","))
    }
}

/**
 * `cargo ndk` for every ABI, writing `<outputDir>/<abi>/libcairn_ffi.so` — the layout AGP takes
 * prebuilt libraries from. It builds with **the NDK AGP resolved** (`ndkDirectory`, from `ndkVersion`),
 * so Gradle and cargo cannot disagree about which NDK is in use. The `cli` feature is never passed:
 * the generator is host-only.
 */
abstract class CargoNdkTask : RustTask() {
    @get:Input
    abstract val ndkDirectory: Property<String>

    @get:Input
    abstract val abis: ListProperty<String>

    @get:Input
    abstract val minSdk: Property<Int>

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val targets = abis.get().flatMap { listOf("-t", it) }
        cargo(
            listOf("ndk") + targets + listOf(
                "-P", minSdk.get().toString(),
                "-o", outputDir.get().asFile.absolutePath,
                "build", "--release", "--lib", "-p", "cairn-ffi",
            ) + featureArgs(),
            // All three names the tools read, so a shell that exported another NDK cannot win.
            env = listOf("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "NDK_HOME").associateWith { ndkDirectory.get() },
        )
    }
}

/**
 * Generates the Kotlin bindings once, on the host, in library mode: it builds the same crate for the
 * host with the `cli` feature (which carries the generator), then runs `uniffi-bindgen` over the host
 * library's metadata. One generated tree serves every ABI. Nothing generated is committed.
 */
abstract class UniffiBindgenTask : RustTask() {
    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        cargo(
            listOf("build", "-p", "cairn-ffi", "--lib", "--bin", "uniffi-bindgen") + featureArgs(listOf("cli")),
        )
        val debug = workspace.get().asFile.resolve("target/debug")
        val library = listOf("libcairn_ffi.so", "libcairn_ffi.dylib").map { debug.resolve(it) }.first { it.exists() }
        val out = outputDir.get().asFile
        out.deleteRecursively()
        out.mkdirs()
        exec.exec {
            workingDir = workspace.get().asFile
            commandLine(
                debug.resolve("uniffi-bindgen").absolutePath, "generate",
                "--library", library.absolutePath,
                "--language", "kotlin",
                "--out-dir", out.absolutePath,
                "--no-format",
            )
        }
    }
}
