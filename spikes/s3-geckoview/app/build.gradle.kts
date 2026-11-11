buildscript {
    // AGP 9 compiles Kotlin itself (built-in Kotlin, bundled KGP 2.2). GeckoView 160 ships kotlin-stdlib 2.4,
    // so the Kotlin compiler is raised by putting a newer KGP on the classpath.
    dependencies { classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.21") }
}

plugins {
    id("com.android.application") version "9.5.1" apply false
}
