plugins {
    id("com.android.application")
}

android {
    namespace = "dev.scyde.spike.gecko"
    compileSdk = 37
    defaultConfig {
        applicationId = "dev.scyde.spike.gecko"
        minSdk = 26
        targetSdk = 36 // 37 once the emulator image exists
        versionCode = 1
        versionName = "0.0.1-spike"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    packaging { resources.excludes += "/META-INF/{AL2.0,LGPL2.1}" }
}

dependencies {
    // Spike: latest GeckoView release; the resolved version is printed by CI and recorded in ADR-0004.
    implementation("org.mozilla.geckoview:geckoview:+")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test:rules:1.6.1")
}
