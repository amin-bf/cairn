// The two tasks that build the Rust core for Android (ADR-0045 §3): plain Gradle API, so this
// module needs no Android or Kotlin plugin of its own.
plugins {
    `kotlin-dsl`
}

repositories {
    mavenCentral()
}
