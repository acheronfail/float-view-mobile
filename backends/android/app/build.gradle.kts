plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

val rustCoreDirectory = rootProject.file("../core")
val rustAndroidHelper = rustCoreDirectory.resolve("vesc-frame-android-jni/build-android.sh")
val rustAndroidOutputs = rustCoreDirectory.resolve("target/android/jniLibs")
val rustHostOutputs = rustCoreDirectory.resolve("target/android/host/debug")
val rustIdentityAndroidHelper =
    rustCoreDirectory.resolve("vesc-firmware-identity-android-jni/build-android.sh")
val rustIdentityAndroidOutputs = rustCoreDirectory.resolve("target/android/identityJniLibs")
val rustIdentityHostOutputs = rustCoreDirectory.resolve("target/android/identity-host/debug")
val rustCustomConfigurationAndroidHelper =
    rustCoreDirectory.resolve("vesc-custom-configuration-android-jni/build-android.sh")
val rustCustomConfigurationAndroidOutputs =
    rustCoreDirectory.resolve("target/android/customConfigurationJniLibs")
val rustCustomConfigurationHostOutputs =
    rustCoreDirectory.resolve("target/android/custom-configuration-host/debug")
val rustBmsValuesAndroidHelper =
    rustCoreDirectory.resolve("vesc-bms-values-android-jni/build-android.sh")
val rustBmsValuesAndroidOutputs =
    rustCoreDirectory.resolve("target/android/bmsValuesJniLibs")
val rustBmsValuesHostOutputs =
    rustCoreDirectory.resolve("target/android/bms-values-host/debug")
val rustHostLibrary = rustHostOutputs.resolve(
    if (System.getProperty("os.name").lowercase().contains("mac")) {
        "libfloat_view_vesc_frame_android_jni.dylib"
    } else {
        "libfloat_view_vesc_frame_android_jni.so"
    }
)
val rustIdentityHostLibrary = rustIdentityHostOutputs.resolve(
    if (System.getProperty("os.name").lowercase().contains("mac")) {
        "libfloat_view_vesc_firmware_identity_android_jni.dylib"
    } else {
        "libfloat_view_vesc_firmware_identity_android_jni.so"
    }
)
val rustCustomConfigurationHostLibrary = rustCustomConfigurationHostOutputs.resolve(
    if (System.getProperty("os.name").lowercase().contains("mac")) {
        "libfloat_view_vesc_custom_configuration_android_jni.dylib"
    } else {
        "libfloat_view_vesc_custom_configuration_android_jni.so"
    }
)
val rustBmsValuesHostLibrary = rustBmsValuesHostOutputs.resolve(
    if (System.getProperty("os.name").lowercase().contains("mac")) {
        "libfloat_view_vesc_bms_values_android_jni.dylib"
    } else {
        "libfloat_view_vesc_bms_values_android_jni.so"
    }
)

fun registerAndroidRustBuild(profile: String) = tasks.register<Exec>(
    "buildAndroidRust${profile.replaceFirstChar(Char::uppercase)}"
) {
    group = "build"
    description = "Builds the four source-owned Android Rust JNI libraries for $profile."
    workingDir(rootProject.projectDir)
    commandLine(
        rustAndroidHelper.absolutePath,
        "android-all",
        profile,
        rustAndroidOutputs.resolve(profile).absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-frame/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-frame-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-frame/src")),
        fileTree(rustCoreDirectory.resolve("vesc-frame-android-jni/src")),
        rustAndroidHelper,
    )
    outputs.dir(rustAndroidOutputs.resolve(profile))
}

val buildAndroidRustDebug = registerAndroidRustBuild("debug")
val buildAndroidRustRelease = registerAndroidRustBuild("release")

fun registerAndroidRustIdentityBuild(profile: String) = tasks.register<Exec>(
    "buildAndroidRustIdentity${profile.replaceFirstChar(Char::uppercase)}"
) {
    group = "build"
    description = "Builds the four source-owned Android Rust identity JNI libraries for $profile."
    workingDir(rootProject.projectDir)
    commandLine(
        rustIdentityAndroidHelper.absolutePath,
        "android-all",
        profile,
        rustIdentityAndroidOutputs.resolve(profile).absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-firmware-identity/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-firmware-identity-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-firmware-identity/src")),
        fileTree(rustCoreDirectory.resolve("vesc-firmware-identity-android-jni/src")),
        rustIdentityAndroidHelper,
    )
    outputs.dir(rustIdentityAndroidOutputs.resolve(profile))
}

val buildAndroidRustIdentityDebug = registerAndroidRustIdentityBuild("debug")
val buildAndroidRustIdentityRelease = registerAndroidRustIdentityBuild("release")

fun registerAndroidRustCustomConfigurationBuild(profile: String) = tasks.register<Exec>(
    "buildAndroidRustCustomConfiguration${profile.replaceFirstChar(Char::uppercase)}"
) {
    group = "build"
    description = "Builds the four source-owned Android Rust custom-configuration JNI libraries for $profile."
    workingDir(rootProject.projectDir)
    commandLine(
        rustCustomConfigurationAndroidHelper.absolutePath,
        "android-all",
        profile,
        rustCustomConfigurationAndroidOutputs.resolve(profile).absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-custom-configuration/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-custom-configuration-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-custom-configuration/src")),
        fileTree(rustCoreDirectory.resolve("vesc-custom-configuration-android-jni/src")),
        rustCustomConfigurationAndroidHelper,
    )
    outputs.dir(rustCustomConfigurationAndroidOutputs.resolve(profile))
}

val buildAndroidRustCustomConfigurationDebug =
    registerAndroidRustCustomConfigurationBuild("debug")
val buildAndroidRustCustomConfigurationRelease =
    registerAndroidRustCustomConfigurationBuild("release")

fun registerAndroidRustBmsValuesBuild(profile: String) = tasks.register<Exec>(
    "buildAndroidRustBmsValues${profile.replaceFirstChar(Char::uppercase)}"
) {
    group = "build"
    description = "Builds the four source-owned Android Rust BMS-values JNI libraries for $profile."
    workingDir(rootProject.projectDir)
    commandLine(
        rustBmsValuesAndroidHelper.absolutePath,
        "android-all",
        profile,
        rustBmsValuesAndroidOutputs.resolve(profile).absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-bms-values/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-bms-values-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-bms-values/src")),
        fileTree(rustCoreDirectory.resolve("vesc-bms-values-android-jni/src")),
        rustBmsValuesAndroidHelper,
    )
    outputs.dir(rustBmsValuesAndroidOutputs.resolve(profile))
}

val buildAndroidRustBmsValuesDebug = registerAndroidRustBmsValuesBuild("debug")
val buildAndroidRustBmsValuesRelease = registerAndroidRustBmsValuesBuild("release")
val buildAndroidRustHost by tasks.registering(Exec::class) {
    group = "verification"
    description = "Builds the exact source-owned host JNI library used by JVM tests."
    workingDir(rootProject.projectDir)
    commandLine(
        rustAndroidHelper.absolutePath,
        "host",
        "debug",
        rustHostOutputs.absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-frame/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-frame-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-frame/src")),
        fileTree(rustCoreDirectory.resolve("vesc-frame-android-jni/src")),
        rustAndroidHelper,
    )
    outputs.file(rustHostLibrary)
}
val buildAndroidRustIdentityHost by tasks.registering(Exec::class) {
    group = "verification"
    description = "Builds the exact source-owned host identity JNI library used by JVM tests."
    workingDir(rootProject.projectDir)
    commandLine(
        rustIdentityAndroidHelper.absolutePath,
        "host",
        "debug",
        rustIdentityHostOutputs.absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-firmware-identity/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-firmware-identity-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-firmware-identity/src")),
        fileTree(rustCoreDirectory.resolve("vesc-firmware-identity-android-jni/src")),
        rustIdentityAndroidHelper,
    )
    outputs.file(rustIdentityHostLibrary)
}
val buildAndroidRustCustomConfigurationHost by tasks.registering(Exec::class) {
    group = "verification"
    description = "Builds the exact source-owned host custom-configuration JNI library used by JVM tests."
    workingDir(rootProject.projectDir)
    commandLine(
        rustCustomConfigurationAndroidHelper.absolutePath,
        "host",
        "debug",
        rustCustomConfigurationHostOutputs.absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-custom-configuration/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-custom-configuration-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-custom-configuration/src")),
        fileTree(rustCoreDirectory.resolve("vesc-custom-configuration-android-jni/src")),
        rustCustomConfigurationAndroidHelper,
    )
    outputs.file(rustCustomConfigurationHostLibrary)
}
val buildAndroidRustBmsValuesHost by tasks.registering(Exec::class) {
    group = "verification"
    description = "Builds the exact source-owned host BMS-values JNI library used by JVM tests."
    workingDir(rootProject.projectDir)
    commandLine(
        rustBmsValuesAndroidHelper.absolutePath,
        "host",
        "debug",
        rustBmsValuesHostOutputs.absolutePath,
    )
    inputs.files(
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("vesc-bms-values/Cargo.toml"),
        rustCoreDirectory.resolve("vesc-bms-values-android-jni/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("vesc-bms-values/src")),
        fileTree(rustCoreDirectory.resolve("vesc-bms-values-android-jni/src")),
        rustBmsValuesAndroidHelper,
    )
    outputs.file(rustBmsValuesHostLibrary)
}
val cleanAndroidRust by tasks.registering(Delete::class) {
    group = "build"
    description = "Deletes generated host and Android Rust JNI outputs."
    delete(rustCoreDirectory.resolve("target/android"))
}

tasks.named("clean") {
    dependsOn(cleanAndroidRust)
}
buildAndroidRustDebug.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustRelease.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustHost.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustIdentityDebug.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustIdentityRelease.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustIdentityHost.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustCustomConfigurationDebug.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustCustomConfigurationRelease.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustCustomConfigurationHost.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustBmsValuesDebug.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustBmsValuesRelease.configure { mustRunAfter(tasks.named("clean")) }
buildAndroidRustBmsValuesHost.configure { mustRunAfter(tasks.named("clean")) }

android {
    namespace = "fail.acheron.floatview"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "fail.acheron.floatview"
        minSdk = 29
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk {
            abiFilters += setOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    sourceSets {
        getByName("main").assets.srcDir("../../../frontend/build")
        getByName("main").assets.srcDir("../../sql")
        getByName("main").assets.srcDir("../../fixtures")
        getByName("debug").jniLibs.srcDir(rustAndroidOutputs.resolve("debug"))
        getByName("release").jniLibs.srcDir(rustAndroidOutputs.resolve("release"))
        getByName("debug").jniLibs.srcDir(rustIdentityAndroidOutputs.resolve("debug"))
        getByName("release").jniLibs.srcDir(rustIdentityAndroidOutputs.resolve("release"))
        getByName("debug").jniLibs.srcDir(rustCustomConfigurationAndroidOutputs.resolve("debug"))
        getByName("release").jniLibs.srcDir(rustCustomConfigurationAndroidOutputs.resolve("release"))
        getByName("debug").jniLibs.srcDir(rustBmsValuesAndroidOutputs.resolve("debug"))
        getByName("release").jniLibs.srcDir(rustBmsValuesAndroidOutputs.resolve("release"))
    }
    testOptions {
        unitTests.all {
            it.dependsOn(
                buildAndroidRustHost,
                buildAndroidRustIdentityHost,
                buildAndroidRustCustomConfigurationHost,
                buildAndroidRustBmsValuesHost,
            )
            it.systemProperty("floatViewVescFrameJniLibrary", rustHostLibrary.absolutePath)
            it.systemProperty(
                "floatViewVescFirmwareIdentityJniLibrary",
                rustIdentityHostLibrary.absolutePath,
            )
            it.systemProperty(
                "floatViewVescCustomConfigurationJniLibrary",
                rustCustomConfigurationHostLibrary.absolutePath,
            )
            it.systemProperty(
                "floatViewVescBmsValuesJniLibrary",
                rustBmsValuesHostLibrary.absolutePath,
            )
            it.systemProperty(
                "floatViewFixtureRoot",
                rootProject.file("../fixtures").absolutePath,
            )
        }
    }
}

tasks.configureEach {
    when (name) {
        "mergeDebugJniLibFolders" -> dependsOn(
            buildAndroidRustDebug,
            buildAndroidRustIdentityDebug,
            buildAndroidRustCustomConfigurationDebug,
            buildAndroidRustBmsValuesDebug,
        )
        "mergeReleaseJniLibFolders" -> dependsOn(
            buildAndroidRustRelease,
            buildAndroidRustIdentityRelease,
            buildAndroidRustCustomConfigurationRelease,
            buildAndroidRustBmsValuesRelease,
        )
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.ktor.server.cio)
    implementation(libs.ktor.server.core)
    implementation(libs.ktor.server.websockets)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.test.runner)
}
