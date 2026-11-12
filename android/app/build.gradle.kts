plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

// Release signing: the owner's keystore when the CI secrets exist (PLAN §13.2), otherwise a
// per-build test key created by CI (installable, but updates need the real key) [OWNER].
val keystoreFile = System.getenv("SCYDE_KEYSTORE")?.let { file(it) }

android {
    namespace = "dev.scyde.browser"
    // GeckoView 160 requires compiling against API 37.2 (ADR-0004).
    compileSdk {
        version = release(37) { minorApiLevel = 2 }
    }
    defaultConfig {
        applicationId = "dev.scyde.browser"
        minSdk = 26
        targetSdk = 36
        versionCode = 4
        versionName = "0.3.0"
    }
    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "armeabi-v7a", "x86_64")
            isUniversalApk = false
        }
    }
    signingConfigs {
        if (keystoreFile != null && keystoreFile.exists()) {
            create("release") {
                storeFile = keystoreFile
                storePassword = System.getenv("SCYDE_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("SCYDE_KEY_ALIAS")
                keyPassword = System.getenv("SCYDE_KEY_PASSWORD")
            }
        }
    }
    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.findByName("release")
        }
    }
    buildFeatures { compose = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    packaging {
        resources.excludes += "/META-INF/{AL2.0,LGPL2.1}"
        jniLibs.useLegacyPackaging = true
    }
}

dependencies {
    implementation("org.mozilla.geckoview:geckoview:160.+")
    implementation(platform("androidx.compose:compose-bom:2026.09.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.13.0")
    // UniFFI Kotlin bindings for libscyde_core (core/ffi).
    implementation("net.java.dev.jna:jna:5.19.1@aar")
}
