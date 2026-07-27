plugins { id("com.android.application") }

android {
    namespace = "dev.leitner.eguislice"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.leitner.eguislice"
        minSdk = 24
        targetSdk = 36
        versionCode = 2
        versionName = "0.2"
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// games-activity 4.4.0 drags in kotlin-stdlib-jdk8 1.6.21 while appcompat 1.7.0 wants
// kotlin-stdlib 1.8.22 — and 1.8 folded the jdk7/jdk8 artifacts into the main one, so the two
// collide with "Duplicate class". Drop the legacy split artifacts.
configurations.all {
    exclude(group = "org.jetbrains.kotlin", module = "kotlin-stdlib-jdk7")
    exclude(group = "org.jetbrains.kotlin", module = "kotlin-stdlib-jdk8")
}

dependencies {
    // The whole point of GameActivity: real IME support via GameTextInput.
    // Note: prefab support must NOT be enabled — android-activity compiles its own C glue.
    implementation("androidx.games:games-activity:4.4.0")
    implementation("androidx.appcompat:appcompat:1.7.0")
}
