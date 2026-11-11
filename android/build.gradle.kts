buildscript {
    // AGP 9 compiles Kotlin itself; GeckoView ships kotlin-stdlib 2.4, so the compiler is raised (ADR-0004).
    dependencies { classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.21") }
}

plugins {
    id("com.android.application") version "9.5.1" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.4.21" apply false
}
