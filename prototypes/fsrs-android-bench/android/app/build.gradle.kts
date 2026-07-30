plugins {
    id("com.android.application")
}

android {
    namespace = "dev.leitner.fsrsbench"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.leitner.fsrsbench"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "0.1"
        ndk {
            // The test handset is arm64-v8a only.
            abiFilters += "arm64-v8a"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
}
