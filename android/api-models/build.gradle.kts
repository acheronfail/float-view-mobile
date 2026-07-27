plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.serialization)
}

sourceSets {
    main {
        kotlin.srcDir("../../api/generated/kotlin")
    }
    test {
        resources.srcDir("../../api/examples")
    }
}

dependencies {
    implementation(libs.kotlinx.serialization.json)
    testImplementation(libs.junit)
}
