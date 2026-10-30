plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.plugin.compose")
}

// The design module (ADR-0045): the tokens, the bundled faces and the theme every screen draws
// through. It depends on nothing of the app, so a screen cannot reach past it for a colour.
android {
    namespace = "dev.cairn.design"
    compileSdk = 36

    defaultConfig {
        minSdk = 24
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildFeatures { compose = true }

    compileOptions {
        sourceCompatibility = JavaVersion.toVersion(property("cairn.bytecode")!!)
        targetCompatibility = JavaVersion.toVersion(property("cairn.bytecode")!!)
    }

    testOptions { unitTests.isReturnDefaultValues = true }
}

// The JDK and bytecode target are `cairn.jdk` and `cairn.bytecode` in gradle.properties.
kotlin { jvmToolchain(property("cairn.jdk").toString().toInt()) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.04.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")

    testImplementation("junit:junit:4.13.2")

    androidTestImplementation(composeBom)
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
