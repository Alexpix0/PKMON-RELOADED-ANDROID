import java.util.Base64
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

// The release key lives beside the project and is never committed (`keystore.properties` is
// ignored), so builds on this machine are signed while the repo stays free of it.
val keystoreProperties = Properties().apply {
    val file = rootProject.file("keystore.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}

// The emulator is a Rust cdylib produced by cargo-ndk:
//   cd src/OpenGMK && cargo ndk -t arm64-v8a --platform 29 build -p gm8emulator --release
// Copy it into jniLibs before packaging, refusing a missing or zeroed-out (unclean shutdown) .so.
val emulatorSo = rootProject.file("../src/OpenGMK/target/aarch64-linux-android/release/libgm8emulator.so")
val jniLibDir = layout.projectDirectory.dir("src/main/jniLibs/arm64-v8a")
val copyEmulatorSo = tasks.register("copyEmulatorSo") {
    inputs.file(emulatorSo).withPropertyName("emulatorSo").optional()
    outputs.dir(jniLibDir)
    doLast {
        if (!emulatorSo.isFile) error("falta ${emulatorSo}; compila con: cargo ndk -t arm64-v8a --platform 29 build -p gm8emulator --release")
        val magic = emulatorSo.inputStream().use { it.readNBytes(4) }
        if (magic.size < 4 || magic[0] != 0x7F.toByte() || magic[1] != 'E'.code.toByte() || magic[2] != 'L'.code.toByte() || magic[3] != 'F'.code.toByte()) {
            error("${emulatorSo} no es un ELF valido (¿quedó a cero tras un corte?)")
        }
        val out = jniLibDir.asFile.apply { mkdirs() }.resolve("libgm8emulator.so")
        emulatorSo.copyTo(out, overwrite = true)
    }
}

android {
    namespace = "com.klisee.reloaded"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.klisee.reloaded"
        minSdk = 29
        targetSdk = 36
        versionCode = 56
        versionName = "1.1.4"

        ndk { abiFilters += "arm64-v8a" }
    }

    signingConfigs {
        create("release") {
            storeFile = file("reloaded_release.jks")
            storePassword = "PokemonReloadedKey2026"
            keyAlias = "reloaded"
            keyPassword = "PokemonReloadedKey2026"
        }
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = false
        }
        debug {
            signingConfig = signingConfigs.getByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    packaging { jniLibs { useLegacyPackaging = false } }
}

tasks.named("preBuild") { dependsOn(copyEmulatorSo) }

dependencies {
    implementation("androidx.core:core-ktx:1.19.0")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.11.0")
    implementation("androidx.compose.ui:ui:1.13.0-alpha01")
    implementation("androidx.compose.ui:ui-graphics:1.13.0-alpha01")
    implementation("androidx.compose.foundation:foundation:1.13.0-alpha01")
    implementation("androidx.compose.runtime:runtime:1.13.0-alpha01")
    implementation("androidx.compose.material3:material3:1.5.0-alpha28")
    implementation("androidx.compose.material:material-icons-extended:1.7.6")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
}
