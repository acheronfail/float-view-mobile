plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

val rustCoreDirectory = rootProject.file("../core")
val rustAndroidHelper = rustCoreDirectory.resolve("build-android-bridge.sh")
val rustBridgeCatalog = rustCoreDirectory.resolve("bridge-build-catalog.sh")
val rustAndroidJniSupportInputs = files(
    rustCoreDirectory.resolve("android-jni-support/Cargo.toml"),
    fileTree(rustCoreDirectory.resolve("android-jni-support/src")),
)

data class RustAndroidBridgeSpec(
    val identifier: String,
    val taskSuffix: String,
    val packageDescription: String,
    val hostDescription: String,
    val crateDirectory: String,
    val androidOutputDirectory: String,
    val hostOutputDirectory: String,
    val libraryStem: String,
    val testLibraryProperty: String,
    val dependencyDirectories: List<String> = emptyList(),
) {
    val jniCrateDirectory = "$crateDirectory-android-jni"

    fun packageTaskName(profile: String) =
        "buildAndroidRust$taskSuffix${profile.replaceFirstChar(Char::uppercase)}"

    val hostTaskName = "buildAndroidRust${taskSuffix}Host"
}

val rustAndroidBridges = listOf(
    RustAndroidBridgeSpec(
        identifier = "frame",
        taskSuffix = "",
        packageDescription = "Builds the four source-owned Android Rust JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host JNI library used by JVM tests.",
        crateDirectory = "vesc-frame",
        androidOutputDirectory = "jniLibs",
        hostOutputDirectory = "host/debug",
        libraryStem = "float_view_vesc_frame_android_jni",
        testLibraryProperty = "floatViewVescFrameJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "firmware-identity",
        taskSuffix = "Identity",
        packageDescription = "Builds the four source-owned Android Rust identity JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host identity JNI library used by JVM tests.",
        crateDirectory = "vesc-firmware-identity",
        androidOutputDirectory = "identityJniLibs",
        hostOutputDirectory = "identity-host/debug",
        libraryStem = "float_view_vesc_firmware_identity_android_jni",
        testLibraryProperty = "floatViewVescFirmwareIdentityJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "custom-configuration",
        taskSuffix = "CustomConfiguration",
        packageDescription = "Builds the four source-owned Android Rust custom-configuration JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host custom-configuration JNI library used by JVM tests.",
        crateDirectory = "vesc-custom-configuration",
        androidOutputDirectory = "customConfigurationJniLibs",
        hostOutputDirectory = "custom-configuration-host/debug",
        libraryStem = "float_view_vesc_custom_configuration_android_jni",
        testLibraryProperty = "floatViewVescCustomConfigurationJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "bms-values",
        taskSuffix = "BmsValues",
        packageDescription = "Builds the four source-owned Android Rust BMS-values JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host BMS-values JNI library used by JVM tests.",
        crateDirectory = "vesc-bms-values",
        androidOutputDirectory = "bmsValuesJniLibs",
        hostOutputDirectory = "bms-values-host/debug",
        libraryStem = "float_view_vesc_bms_values_android_jni",
        testLibraryProperty = "floatViewVescBmsValuesJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "controller-data",
        taskSuffix = "ControllerData",
        packageDescription = "Builds the source-owned Android Rust controller-data JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host controller-data JNI library used by JVM tests.",
        crateDirectory = "vesc-controller-data",
        androidOutputDirectory = "controllerDataJniLibs",
        hostOutputDirectory = "controller-data-host/debug",
        libraryStem = "float_view_vesc_controller_data_android_jni",
        testLibraryProperty = "floatViewVescControllerDataJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "refloat-control",
        taskSuffix = "RefloatControl",
        packageDescription = "Builds the four source-owned Android Rust Refloat-control JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host Refloat-control JNI library used by JVM tests.",
        crateDirectory = "refloat-control",
        androidOutputDirectory = "refloatControlJniLibs",
        hostOutputDirectory = "refloat-control-host/debug",
        libraryStem = "float_view_refloat_control_android_jni",
        testLibraryProperty = "floatViewRefloatControlJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "refloat-compatibility",
        taskSuffix = "RefloatCompatibility",
        packageDescription = "Builds the four source-owned Android Rust Refloat compatibility JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host Refloat compatibility JNI library used by JVM tests.",
        crateDirectory = "refloat-compatibility",
        androidOutputDirectory = "refloatCompatibilityJniLibs",
        hostOutputDirectory = "refloat-compatibility-host/debug",
        libraryStem = "float_view_refloat_compatibility_android_jni",
        testLibraryProperty = "floatViewRefloatCompatibilityJniLibrary",
        dependencyDirectories = listOf("refloat-dynamic-telemetry"),
    ),
    RustAndroidBridgeSpec(
        identifier = "refloat-dynamic-telemetry",
        taskSuffix = "RefloatDynamicTelemetry",
        packageDescription = "Builds the four source-owned Android Rust Refloat dynamic telemetry JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host Refloat dynamic telemetry JNI library used by JVM tests.",
        crateDirectory = "refloat-dynamic-telemetry",
        androidOutputDirectory = "refloatDynamicTelemetryJniLibs",
        hostOutputDirectory = "refloat-dynamic-telemetry-host/debug",
        libraryStem = "float_view_refloat_dynamic_telemetry_android_jni",
        testLibraryProperty = "floatViewRefloatDynamicTelemetryJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "refloat-legacy-telemetry",
        taskSuffix = "RefloatLegacyTelemetry",
        packageDescription = "Builds the four source-owned Android Rust Refloat legacy telemetry JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host Refloat legacy telemetry JNI library used by JVM tests.",
        crateDirectory = "refloat-legacy-telemetry",
        androidOutputDirectory = "refloatLegacyTelemetryJniLibs",
        hostOutputDirectory = "refloat-legacy-telemetry-host/debug",
        libraryStem = "float_view_refloat_legacy_telemetry_android_jni",
        testLibraryProperty = "floatViewRefloatLegacyTelemetryJniLibrary",
    ),
    RustAndroidBridgeSpec(
        identifier = "refloat-configuration-codec",
        taskSuffix = "RefloatConfigurationCodec",
        packageDescription = "Builds the four source-owned Android Rust Refloat configuration-codec JNI libraries for {profile}.",
        hostDescription = "Builds the exact source-owned host Refloat configuration-codec JNI library used by JVM tests.",
        crateDirectory = "refloat-configuration-codec",
        androidOutputDirectory = "refloatConfigurationCodecJniLibs",
        hostOutputDirectory = "refloat-configuration-codec-host/debug",
        libraryStem = "float_view_refloat_configuration_codec_android_jni",
        testLibraryProperty = "floatViewRefloatConfigurationCodecJniLibrary",
    ),
)

fun requireUniqueRustBridgeField(name: String, values: List<String>) {
    require(values.distinct().size == values.size) {
        "Android Rust bridge $name values must be unique: $values"
    }
}

require(rustAndroidBridges.size == 10) { "Expected exactly ten Android Rust bridges." }
require(
    rustAndroidBridges.map { it.identifier }.toSet() == setOf(
        "frame",
        "firmware-identity",
        "custom-configuration",
        "bms-values",
        "controller-data",
        "refloat-control",
        "refloat-compatibility",
        "refloat-dynamic-telemetry",
        "refloat-legacy-telemetry",
        "refloat-configuration-codec",
    )
) { "Android Rust bridge identifiers must match the shared shell catalog." }
requireUniqueRustBridgeField("task suffix", rustAndroidBridges.map { it.taskSuffix })
requireUniqueRustBridgeField("crate directory", rustAndroidBridges.map { it.crateDirectory })
requireUniqueRustBridgeField("Android output", rustAndroidBridges.map { it.androidOutputDirectory })
requireUniqueRustBridgeField("host output", rustAndroidBridges.map { it.hostOutputDirectory })
requireUniqueRustBridgeField("library stem", rustAndroidBridges.map { it.libraryStem })
requireUniqueRustBridgeField("test property", rustAndroidBridges.map { it.testLibraryProperty })
requireUniqueRustBridgeField(
    "generated output",
    rustAndroidBridges.flatMap { listOf(it.androidOutputDirectory, it.hostOutputDirectory) },
)

val rustHostLibraryExtension =
    if (System.getProperty("os.name").lowercase().contains("mac")) "dylib" else "so"

fun RustAndroidBridgeSpec.androidOutputs() =
    rustCoreDirectory.resolve("target/android/$androidOutputDirectory")

fun RustAndroidBridgeSpec.hostOutputs() =
    rustCoreDirectory.resolve("target/android/$hostOutputDirectory")

fun RustAndroidBridgeSpec.hostLibrary() =
    hostOutputs().resolve("lib$libraryStem.$rustHostLibraryExtension")

fun RustAndroidBridgeSpec.inputFiles() =
    files(
        rustAndroidJniSupportInputs,
        rustCoreDirectory.resolve("Cargo.toml"),
        rustCoreDirectory.resolve("Cargo.lock"),
        rustCoreDirectory.resolve("rust-toolchain.toml"),
        rustCoreDirectory.resolve("$crateDirectory/Cargo.toml"),
        rustCoreDirectory.resolve("$jniCrateDirectory/Cargo.toml"),
        fileTree(rustCoreDirectory.resolve("$crateDirectory/src")),
        fileTree(rustCoreDirectory.resolve("$jniCrateDirectory/src")),
        dependencyDirectories.map { directory ->
            rustCoreDirectory.resolve("$directory/Cargo.toml")
        },
        dependencyDirectories.map { directory ->
            fileTree(rustCoreDirectory.resolve("$directory/src"))
        },
        rustAndroidHelper,
        rustBridgeCatalog,
    )

fun Exec.registerRustBridgeInputs(spec: RustAndroidBridgeSpec) {
    inputs.files(spec.inputFiles())
}

fun registerAndroidRustPackageBuild(spec: RustAndroidBridgeSpec, profile: String) =
    tasks.register<Exec>(spec.packageTaskName(profile)) {
        group = "build"
        description = spec.packageDescription.replace("{profile}", profile)
        workingDir(rootProject.projectDir)
        commandLine(
            rustAndroidHelper.absolutePath,
            spec.identifier,
            "android-all",
            profile,
            spec.androidOutputs().resolve(profile).absolutePath,
        )
        registerRustBridgeInputs(spec)
        outputs.dir(spec.androidOutputs().resolve(profile))
    }

fun registerAndroidRustHostBuild(spec: RustAndroidBridgeSpec) =
    tasks.register<Exec>(spec.hostTaskName) {
        group = "verification"
        description = spec.hostDescription
        workingDir(rootProject.projectDir)
        commandLine(
            rustAndroidHelper.absolutePath,
            spec.identifier,
            "host",
            "debug",
            spec.hostOutputs().absolutePath,
        )
        registerRustBridgeInputs(spec)
        outputs.file(spec.hostLibrary())
    }

val rustAndroidBuildTasks = listOf("debug", "release").associateWith { profile ->
    rustAndroidBridges.associate { spec ->
        spec.identifier to registerAndroidRustPackageBuild(spec, profile)
    }
}
val rustHostBuildTasks = rustAndroidBridges.associate { spec ->
    spec.identifier to registerAndroidRustHostBuild(spec)
}
val rustBuildTaskProviders =
    rustAndroidBuildTasks.values.flatMap { it.values } + rustHostBuildTasks.values
val expectedRustBuildTaskNames = rustAndroidBridges.flatMap { spec ->
    listOf(spec.packageTaskName("debug"), spec.packageTaskName("release"), spec.hostTaskName)
}.toSet()

require(expectedRustBuildTaskNames.size == 30) {
    "Expected exactly thirty unique Android Rust bridge task names."
}

val cleanAndroidRust by tasks.registering(Delete::class) {
    group = "build"
    description = "Deletes generated host and Android Rust JNI outputs."
    delete(rustCoreDirectory.resolve("target/android"))
}

tasks.named("clean") {
    dependsOn(cleanAndroidRust)
}
rustBuildTaskProviders.forEach { provider ->
    provider.configure { mustRunAfter(tasks.named("clean")) }
}

val auditAndroidRustBridgeTasks by tasks.registering {
    group = "verification"
    description = "Audits the closed Android Rust Gradle bridge task catalog."
    doLast {
        val actualTaskNames = tasks.names.filter { it.startsWith("buildAndroidRust") }.toSet()
        require(actualTaskNames == expectedRustBuildTaskNames) {
            "Android Rust task surface drifted: expected=$expectedRustBuildTaskNames actual=$actualTaskNames"
        }
        rustAndroidBridges.forEach { spec ->
            listOf("debug", "release").forEach { profile ->
                val task = rustAndroidBuildTasks.getValue(profile).getValue(spec.identifier).get()
                require(task.group == "build")
                require(task.description == spec.packageDescription.replace("{profile}", profile))
                require(
                    task.commandLine == listOf(
                        rustAndroidHelper.absolutePath,
                        spec.identifier,
                        "android-all",
                        profile,
                        spec.androidOutputs().resolve(profile).absolutePath,
                    )
                ) { "Android Rust ${spec.identifier} $profile command drifted." }
                require(task.inputs.files.files == spec.inputFiles().files) {
                    "Android Rust ${spec.identifier} $profile inputs drifted."
                }
                require(task.outputs.files.files == setOf(spec.androidOutputs().resolve(profile))) {
                    "Android Rust ${spec.identifier} $profile outputs drifted."
                }
            }
            val hostTask = rustHostBuildTasks.getValue(spec.identifier).get()
            require(hostTask.group == "verification")
            require(hostTask.description == spec.hostDescription)
            require(
                hostTask.commandLine == listOf(
                    rustAndroidHelper.absolutePath,
                    spec.identifier,
                    "host",
                    "debug",
                    spec.hostOutputs().absolutePath,
                )
            ) { "Android Rust ${spec.identifier} host command drifted." }
            require(hostTask.inputs.files.files == spec.inputFiles().files) {
                "Android Rust ${spec.identifier} host inputs drifted."
            }
            require(hostTask.outputs.files.files == setOf(spec.hostLibrary())) {
                "Android Rust ${spec.identifier} host output drifted."
            }
        }
    }
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
        rustAndroidBridges.forEach { spec ->
            getByName("debug").jniLibs.srcDir(spec.androidOutputs().resolve("debug"))
            getByName("release").jniLibs.srcDir(spec.androidOutputs().resolve("release"))
        }
    }
    testOptions {
        unitTests.all {
            it.dependsOn(rustHostBuildTasks.values)
            rustAndroidBridges.forEach { spec ->
                it.systemProperty(spec.testLibraryProperty, spec.hostLibrary().absolutePath)
            }
            it.systemProperty(
                "floatViewFixtureRoot",
                rootProject.file("../fixtures").absolutePath,
            )
        }
    }
}

tasks.configureEach {
    when (name) {
        "mergeDebugJniLibFolders" -> dependsOn(rustAndroidBuildTasks.getValue("debug").values)
        "mergeReleaseJniLibFolders" -> dependsOn(rustAndroidBuildTasks.getValue("release").values)
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
