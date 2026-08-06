/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

plugins {
    `kotlin-dsl`
    alias(libs.plugins.spotless)
}

val mozconfig = gradle.extra["mozconfig"] as Map<*, *>
val topsrcdir = mozconfig["topsrcdir"] as String

spotless {
    lineEndings = com.diffplug.spotless.LineEnding.UNIX
    kotlin {
        ktlint(libs.versions.ktlint.get())
            .setEditorConfigPath("$topsrcdir/mobile/android/geckoview/.editorconfig")
    }
}

sourceSets {
    main {
        resources {
            output.dir(mapOf("builtBy" to "copyDocletJar"), layout.buildDirectory.dir("docletJar"))
        }
    }
}

gradlePlugin {
    plugins.register("apilintPlugin") {
        id = "org.mozilla.apilint"
        displayName = "API Lint plugin"
        description = "Tracks the API of an Android library and helps maintain backward compatibility."
        implementationClass = "org.mozilla.apilint.ApiLintPlugin"
    }
}

tasks.register<Exec>("testApiLint") {
    workingDir(".")
    commandLine("python3", "src/test/resources/apilint_test.py",
        "--build-dir", layout.buildDirectory.get().asFile)
}

tasks.register<Exec>("unittestApiLint") {
    workingDir(".")
    commandLine("python3", "src/test/resources/apilint_unittest.py")
}

tasks.register<Exec>("testChangelogCheck") {
    workingDir(".")
    commandLine("python3", "src/test/resources/changelog-check_test.py")
}

// Tests that the expected doclet result is understood by apilint.py
tasks.register<Exec>("integrationTestApiLint") {
    workingDir(".")
    commandLine("python3", "src/main/resources/apilint.py",
         "../apidoc-plugin/src/test/resources/expected-doclet-output.txt",
         "../apidoc-plugin/src/test/resources/expected-doclet-output.txt")
}

tasks.named<Test>("test") {
    useJUnitPlatform()

    dependsOn("spotlessCheck")
    dependsOn("unittestApiLint")
    dependsOn("testApiLint")
    dependsOn("testChangelogCheck")
    dependsOn("integrationTestApiLint")
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    compilerOptions {
        allWarningsAsErrors.set(true)
    }
}

// Arrange for the doclet jar to be included in Java resources, to be consumed
// at runtime.
val docletJar = configurations.create("docletJar")

dependencies {
    compileOnly(libs.android.gradle.plugin)
    docletJar(project(path = ":apidoc-plugin", configuration = "docletJar"))

    testImplementation(platform(libs.junit.bom))
    testImplementation(libs.junit.jupiter)
    testRuntimeOnly(libs.junit.platform.launcher)
}

tasks.register<Sync>("copyDocletJar") {
    from(configurations.named("docletJar"))
    into(layout.buildDirectory.dir("docletJar"))
}
