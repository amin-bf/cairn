import com.android.build.api.variant.ApplicationAndroidComponentsExtension

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

// Read through Gradle's providers, so the variable is a configuration-cache input; independent of the
// build type. Only the exact value `1` compiles the Development block in.
val developmentBuild = providers.environmentVariable("CAIRN_DEVELOPMENT").orNull == "1"

// The ABIs the build packages. `x86_64` is the emulator, `arm64-v8a` the handset (ADR-0045).
val targetAbis = listOf("arm64-v8a", "x86_64")

android {
    namespace = "dev.cairn.app"
    compileSdk = 36
    // Pinned, never AGP's default: the binding was measured on this NDK, and the cargo-ndk task below
    // builds with the directory AGP resolves from it.
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "dev.cairn.app"
        minSdk = 24
        targetSdk = 36
        // ADR-0045 §4: an upgrade over the retired egui build (§8), whose emitted manifest carried
        // 16777472 (its packager derived it from the crate version, 0.1.0). Android refuses an
        // install -r that does not raise it. Debug builds are signed with ~/.android/debug.keystore,
        // which is the key the egui build's packager signed with too, so the upgrade keeps the data.
        versionCode = 20_000_000
        versionName = "0.2.0"
        ndk { abiFilters += targetAbis }
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildFeatures { compose = true }

    buildTypes {
        release {
            // R8 shrinks and renames; app/proguard-rules.pro keeps what native code binds by name.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            // Signing a real release key is not part of this build. The release is signed with the
            // debug key so it can be installed and run; a shipping key replaces this.
            signingConfig = signingConfigs.getByName("debug")
            testProguardFiles("proguard-test-rules.pro")
        }
    }

    // The release build exactly, plus the keeps the test runner needs in the app's own dex (it shares
    // the process, and R8 would otherwise strip what only the runner calls). `-Pcairn.testMinified`
    // runs the instrumented tests against it, which is how the keep rules are proven
    // (docs/environment/android-toolchain.md); the shipped `release` carries none of the test keeps.
    buildTypes.create("minified") {
        initWith(buildTypes.getByName("release"))
        matchingFallbacks += "release"
        proguardFiles("proguard-minified-rules.pro")
    }
    if (providers.gradleProperty("cairn.testMinified").isPresent) testBuildType = "minified"

    // The launcher mark is the retired egui build's, shared rather than copied: the upgrade keeps its
    // icon, and scripts/build-icon-face.py generates the desktop's icon face from the same drawable.
    sourceSets["main"].res.directories.add("../../crates/app/res")

    // ADR-0051 §1: the Development block exists only in a build made with CAIRN_DEVELOPMENT=1. Its
    // Kotlin, its strings and its tests are a source set added only then; otherwise a source set of the
    // same shape supplies a block that draws nothing, so the code and its strings are not in the APK.
    // Both builds keep the package id and the signing key, so one installs over the other (§2).
    val flavour = if (developmentBuild) "development" else "shipping"
    sourceSets["main"].kotlin.directories.add("src/$flavour/kotlin")
    sourceSets["main"].res.directories.add("src/$flavour/res")
    sourceSets["androidTest"].kotlin.directories.add("src/androidTest${flavour.replaceFirstChar { it.uppercase() }}/kotlin")

    compileOptions {
        val bytecode = JavaVersion.toVersion(property("cairn.bytecode").toString())
        sourceCompatibility = bytecode
        targetCompatibility = bytecode
    }

    packaging {
        // The Rust library is stored uncompressed and page aligned (16 KB), as AGP does by default.
        jniLibs { useLegacyPackaging = false }
    }
}

// The JDK and bytecode target are `cairn.jdk` and `cairn.bytecode` in gradle.properties.
kotlin { jvmToolchain(property("cairn.jdk").toString().toInt()) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.04.01")
    implementation(composeBom)
    implementation(project(":design"))
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.activity:activity-compose:1.12.4")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.10.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.10.0")
    implementation("androidx.navigation:navigation-compose:2.9.7")

    // What the uniffi-generated Kotlin needs (research §1): JNA as an aar, which carries
    // libjnidispatch.so per ABI, coroutines, and androidx.annotation for `android = true`.
    implementation("net.java.dev.jna:jna:5.19.1@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    implementation("androidx.annotation:annotation:1.9.1")

    androidTestImplementation(composeBom)
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.10.2")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

// ADR-0045 §3: one `assemble` builds the Rust core for every ABI, generates the Kotlin bindings and
// packages both. The two tasks are registered through the Variant API and their outputs joined to the
// `jniLibs` and Kotlin source sets, so AGP orders them before packaging with no hand-written
// `dependsOn`, and nothing generated is committed.
val repositoryRoot = rootProject.layout.projectDirectory.dir("..")

val androidComponents = extensions.getByType<ApplicationAndroidComponentsExtension>()

androidComponents.onVariants { variant ->
    val name = variant.name.replaceFirstChar { it.uppercase() }
    // `panic-probe` is the one export that panics on purpose, for the test that proves a Rust panic
    // reaches Kotlin as an exception. A release build never carries it.
    val variantFeatures = if (variant.buildType == "debug") listOf("panic-probe") else emptyList()

    val cargoNdk = tasks.register<CargoNdkTask>("cargoNdk$name") {
        workspace.set(repositoryRoot)
        features.set(variantFeatures)
        ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory.map { it.asFile.absolutePath })
        abis.set(targetAbis)
        minSdk.set(24)
        outputDir.set(layout.buildDirectory.dir("generated/cairn/jniLibs/${variant.name}"))
    }
    variant.sources.jniLibs!!.addGeneratedSourceDirectory(cargoNdk, CargoNdkTask::outputDir)

    val bindgen = tasks.register<UniffiBindgenTask>("uniffiBindgen$name") {
        workspace.set(repositoryRoot)
        features.set(variantFeatures)
        outputDir.set(layout.buildDirectory.dir("generated/cairn/kotlin/${variant.name}"))
    }
    variant.sources.kotlin!!.addGeneratedSourceDirectory(bindgen, UniffiBindgenTask::outputDir)
}
