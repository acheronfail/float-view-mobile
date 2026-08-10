plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

val rustCoreDirectory = rootProject.file("../core")
val rustAndroidHelper = rustCoreDirectory.resolve("vesc-frame-android-jni/build-android.sh")
val rustAndroidOutputs = rustCoreDirectory.resolve("target/android/jniLibs")
val rustHostOutputs = rustCoreDirectory.resolve("target/android/host/debug")
val rustHostLibrary = rustHostOutputs.resolve(
    if (System.getProperty("os.name").lowercase().contains("mac")) {
        "libfloat_view_vesc_frame_android_jni.dylib"
    } else {
        "libfloat_view_vesc_frame_android_jni.so"
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
    }
    testOptions {
        unitTests.all {
            it.dependsOn(buildAndroidRustHost)
            it.systemProperty("floatViewVescFrameJniLibrary", rustHostLibrary.absolutePath)
            it.systemProperty(
                "floatViewFixtureRoot",
                rootProject.file("../fixtures").absolutePath,
            )
        }
    }
}

tasks.configureEach {
    when (name) {
        "mergeDebugJniLibFolders" -> dependsOn(buildAndroidRustDebug)
        "mergeReleaseJniLibFolders" -> dependsOn(buildAndroidRustRelease)
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
