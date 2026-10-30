//! The Kotlin generator, run on the host by the Gradle build (`--features cli`). Not built for Android.

fn main() {
    uniffi::uniffi_bindgen_main()
}
