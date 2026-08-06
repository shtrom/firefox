/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

plugins {
    java
    alias(libs.plugins.spotless)
}

val javadocExecutable = "${System.getProperty("java.home")}/bin/javadoc"

val testApiDoclet = tasks.register<Exec>("testApiDoclet") {
    val jarTask = tasks.named<Jar>("jar")
    val docletJar = jarTask.flatMap { it.archiveFile }
    dependsOn(jarTask)

    workingDir(".")
    commandLine(
        "python3", file("src/test/resources/apidoc_test.py"),
        "--javadoc", javadocExecutable,
        "--doclet-jar", docletJar.get().asFile.absolutePath,
        "--java-root", file("src/test/fake_root"),
        "--out-dir", layout.buildDirectory.dir("tmp").get().asFile,
        "--expected", file("src/test/resources/expected-doclet-output.txt"),
        "--expected-map", file("src/test/resources/expected-map-output.txt"),
    )
}

tasks.named<Test>("test") {
    dependsOn(testApiDoclet)
    dependsOn("spotlessCheck")
}

spotless {
    lineEndings = com.diffplug.spotless.LineEnding.UNIX
    java {
        googleJavaFormat(libs.versions.google.java.format.get())
    }
}

configurations.create("docletJar")

artifacts {
    add("docletJar", tasks.named<Jar>("jar"))
}
