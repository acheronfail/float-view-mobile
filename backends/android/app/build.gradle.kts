import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

val rustCoreScript = rootProject.file("scripts/rust-core.sh")
val rustJniDirectory = layout.buildDirectory.dir("generated/rustJni")
val localProperties = Properties().apply {
    val file = rootProject.file("local.properties")
    if (file.isFile) {
        file.inputStream().use { stream -> load(stream) }
    }
}
val androidSdkRoot = providers.environmentVariable("ANDROID_SDK_ROOT")
    .orElse(providers.environmentVariable("ANDROID_HOME"))
    .orNull
    ?: localProperties.getProperty("sdk.dir")
    ?: error("ANDROID_SDK_ROOT, ANDROID_HOME, or local.properties sdk.dir must identify the Android SDK")
fun Exec.configureRustCoreCommand(vararg arguments: String) {
    environment("ANDROID_SDK_ROOT", androidSdkRoot)
    commandLine("sh", rustCoreScript.absolutePath, *arguments)
}

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
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
        }
    }
    buildFeatures {
        buildConfig = true
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
        getByName("main").jniLibs.srcDir(rustJniDirectory)
    }
    testOptions {
        unitTests.all {
            it.systemProperty(
                "floatViewFixtureRoot",
                rootProject.file("../fixtures").absolutePath,
            )
        }
    }
}

val rustCoreInputs = fileTree("../../core") {
    include(
        "Cargo.toml",
        "Cargo.lock",
        "checksums.sha256",
        "include/float_view_core.h",
        "crates/**/*.rs",
        "crates/**/Cargo.toml",
    )
    exclude("target/**")
}

val validateAndroidRustToolchain by tasks.registering(Exec::class) {
    group = "verification"
    description = "Validates the locked Rust, JDK, NDK, Cargo, header, and dependency inputs."
    inputs.files(rustCoreScript, rustCoreInputs)
    configureRustCoreCommand("validate")
}

val androidRustAbis = listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
val rustAbiBuilds = androidRustAbis.map { abi ->
    val taskSuffix = abi.split('-', '_').joinToString("") { part ->
        part.replaceFirstChar { character -> character.uppercase() }
    }
    tasks.register<Exec>("buildRustCore$taskSuffix") {
        group = "build"
        description = "Builds the locked Rust JNI library once for Android $abi."
        dependsOn(validateAndroidRustToolchain)
        inputs.files(rustCoreScript, rustCoreInputs)
        outputs.file(rustJniDirectory.map { it.file("$abi/libhost_android_jni.so") })
        configureRustCoreCommand(
            "build",
            abi,
            rustJniDirectory.get().asFile.absolutePath,
        )
    }
}

val buildRustCore by tasks.registering {
    group = "build"
    description = "Builds one locked Rust JNI library for every packaged Android ABI."
    dependsOn(rustAbiBuilds)
}

val verifyRustCorePackaging by tasks.registering(Exec::class) {
    group = "verification"
    description = "Rejects duplicate, wrong-ABI, wrong-identity, or wrongly exported Rust libraries."
    dependsOn(buildRustCore)
    inputs.files(rustAbiBuilds.map { task -> task.flatMap { it.outputs.files.elements } })
    outputs.file(rustJniDirectory.map { it.file("manifest.sha256") })
    configureRustCoreCommand("verify", rustJniDirectory.get().asFile.absolutePath)
}

tasks.named("preBuild").configure {
    dependsOn(verifyRustCorePackaging)
}

dependencies {
    implementation(libs.androidx.core.ktx)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.test.runner)
}
