/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.apilint

import com.android.build.api.variant.LibraryAndroidComponentsExtension
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.tasks.Copy
import org.gradle.api.tasks.TaskProvider
import org.gradle.api.tasks.compile.JavaCompile

class ApiLintPlugin : Plugin<Project> {
    override fun apply(project: Project) {
        val extension = project.extensions.create("apiLint", ApiLintPluginExtension::class.java)

        project.pluginManager.withPlugin("com.android.library") {
            val docletJarFile = project.layout.buildDirectory.file("docletJar/apidoc-plugin.jar")
            val resourceName = "apidoc-plugin.jar"

            val copyDocletJarResource = project.tasks.register("copyDocletJarResource") {
                inputs.property("resourceName", resourceName)
                outputs.file(docletJarFile)
                doLast {
                    val resourceStream = ApiLintPlugin::class.java.classLoader.getResourceAsStream(resourceName)
                        ?: throw RuntimeException("Java resource not found: $resourceName")
                    resourceStream.use { input ->
                        outputs.files.singleFile.outputStream().use { out ->
                            input.copyTo(out)
                        }
                    }
                }
            }

            // The compile classpath is taken from the variant's javac task, but AGP creates that
            // task after the onVariants callbacks run, so we defer wiring it until afterEvaluate.
            val apiGenerateTasks = mutableMapOf<String, TaskProvider<ApiCompatLintTask>>()

            val androidComponents =
                project.extensions.getByType(LibraryAndroidComponentsExtension::class.java)
            androidComponents.onVariants(androidComponents.selector().all()) { variant ->
                val variantName = variant.name
                val name = variantName.replaceFirstChar { c -> c.titlecase() }

                // The generated API files used to live in the variant's javac output directory.
                // The new variant API does not expose that directory at configuration time, so we
                // write them to a dedicated, variant-scoped directory instead.
                val outputDir = project.layout.buildDirectory.dir("apilint/${variantName}")
                val apiFileProvider = outputDir.flatMap { dir -> extension.apiOutputFileName.map { dir.file(it) } }
                val jsonResultFileProvider =
                    outputDir.flatMap { dir -> extension.jsonResultFileName.map { dir.file(it) } }
                val currentApiFileProvider = project.layout.projectDirectory.file(extension.currentApiRelativeFilePath)
                val apiMapFileProvider = apiMapFileFor(project.layout, apiFileProvider)

                // sources.java.all covers the static sources plus the AGP-generated BuildConfig/AIDL
                // sources (replacing the legacy sourceSets/generateBuildConfig/aidlCompile accessors).
                // Generated non-API types (BuildConfig, R) are filtered by skipClassesRegex/exclude below.
                val javaSources = variant.sources.java ?: return@onVariants
                val sourceDirs = javaSources.all

                val apiGenerate = project.tasks.register("apiGenerate${name}", ApiCompatLintTask::class.java) {
                    description = "Generates API file for build variant ${name}"
                    dependsOn(copyDocletJarResource)

                    setSource(sourceDirs)
                    exclude("**/R.java")
                    include("**/**.java")

                    sourcePath.from(sourceDirs)

                    rootDir.set(project.rootDir.absolutePath)
                    outputFile.set(apiFileProvider)
                    apiMapFile.set(apiMapFileProvider)
                    packageFilter.set(extension.packageFilter)
                    skipClassesRegex.set(extension.skipClassesRegex)
                    javadocDestinationDir.set(project.layout.buildDirectory.dir("tmp/javadoc/${variantName}"))
                    docletPath.set(docletJarFile)
                }
                apiGenerateTasks[variantName] = apiGenerate

                val apiLintSingle = project.tasks.register("apiLintSingle${name}", PythonExec::class.java) {
                    description = "Runs API lint checks for variant ${name}"
                    dependsOn(apiGenerate)
                    scriptPath.set("apilint.py")

                    inputs.file(apiFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                    inputs.file(apiMapFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                    declareLintFilterInputs(extension)
                    declareDeprecationInputs(extension)
                    outputs.file(jsonResultFileProvider)

                    doFirst {
                        val apiFile = apiFileProvider.get().asFile
                        val jsonResultFile = jsonResultFileProvider.get().asFile
                        val apiMapFile = apiMapFileProvider.get().asFile

                        args(apiFile, "--result-json", jsonResultFile)
                        // Gradle gives a ListProperty an empty value rather than no value, so these
                        // have to be checked for emptiness: `isPresent` is true even when the
                        // consumer never configured them, and passing either flag with no values
                        // makes apilint.py restrict the API to nothing.
                        val lintFilters = extension.lintFilters.get()
                        if (lintFilters.isNotEmpty()) {
                            args("--filter-errors", *lintFilters.toTypedArray())
                        }
                        val allowedPackages = extension.allowedPackages.get()
                        if (allowedPackages.isNotEmpty()) {
                            args("--allowed-packages", *allowedPackages.toTypedArray())
                        }
                        if (extension.deprecationAnnotation.isPresent) {
                            args("--deprecation-annotation", extension.deprecationAnnotation.get())
                        }
                        if (extension.libraryVersion.isPresent) {
                            args("--library-version", extension.libraryVersion.get())
                        }
                        args("--api-map", apiMapFile)
                    }
                }

                val apiDiff = project.tasks.register("apiDiff${name}", PythonExec::class.java) {
                    description = "Prints the diff between the existing API and the local API."
                    group = "Verification"
                    dependsOn(apiGenerate)
                    scriptPath.set("diff.py")

                    inputs.file(apiFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                    inputs.file(currentApiFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)

                    // diff exit value is != 0 if the files are different
                    isIgnoreExitValue = true

                    doFirst {
                        val apiFile = apiFileProvider.get().asFile
                        val currentApiFile = currentApiFileProvider.get().asFile

                        args("--existing", currentApiFile, "--local", apiFile, "--command", extension.helpCommand.get()(name))
                    }
                }

                val apiCompatLint = project.tasks.register("apiCompatLint${name}", PythonExec::class.java) {
                    description = "Runs API compatibility lint checks for variant ${name}"
                    scriptPath.set("apilint.py")

                    inputs.file(apiFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                    inputs.file(currentApiFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                    inputs.file(apiMapFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                    declareDeprecationInputs(extension)
                    outputs.file(jsonResultFileProvider)

                    dependsOn(apiLintSingle)
                    finalizedBy(apiDiff)

                    doFirst {
                        val apiFile = apiFileProvider.get().asFile
                        val jsonResultFile = jsonResultFileProvider.get().asFile
                        val currentApiFile = currentApiFileProvider.get().asFile
                        val apiMapFile = apiMapFileProvider.get().asFile

                        args("--show-noticed", apiFile, currentApiFile, "--result-json", jsonResultFile, "--append-json", "--api-map", apiMapFile)
                        if (extension.deprecationAnnotation.isPresent) {
                            args("--deprecation-annotation", extension.deprecationAnnotation.get())
                        }
                        if (extension.libraryVersion.isPresent) {
                            args("--library-version", extension.libraryVersion.get())
                        }
                    }
                }

                val lintDependency = if (extension.changelogFileName.isPresent) {
                    val changelogFileProvider = project.layout.projectDirectory.file(extension.changelogFileName)
                    project.tasks.register("apiChangelogCheck${name}", PythonExec::class.java) {
                        description = "Checks that the API changelog has been updated."
                        group = "Verification"
                        scriptPath.set("changelog-check.py")

                        inputs.file(apiFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                        inputs.file(changelogFileProvider).withPathSensitivity(org.gradle.api.tasks.PathSensitivity.RELATIVE)
                        outputs.file(jsonResultFileProvider)

                        dependsOn(apiCompatLint)

                        doFirst {
                            val apiFile = apiFileProvider.get().asFile
                            val jsonResultFile = jsonResultFileProvider.get().asFile
                            val changelogFile = changelogFileProvider.get().asFile

                            args("--api-file", apiFile, "--changelog-file", changelogFile, "--result-json", jsonResultFile)
                        }
                    }
                } else {
                    apiCompatLint
                }

                val apiLint = project.tasks.register("apiLint${name}") {
                    description = "Runs API lint checks for variant ${name}"
                    group = "Verification"
                    dependsOn(lintDependency)
                }

                project.tasks.named("check") {
                    dependsOn(apiLint)
                }

                project.tasks.register("apiUpdateFile${name}", Copy::class.java) {
                    description = "Updates the API file from the local one for variant ${name}"
                    group = "Verification"
                    dependsOn(apiGenerate)
                    from(apiFileProvider)
                    into(currentApiFileProvider.map { it.asFile.parentFile })
                    rename { currentApiFileProvider.get().asFile.name }
                }
            }

            // AGP creates the variant javac tasks after onVariants runs, so wire each
            // apiGenerate task's classpath from the corresponding javac task here.
            project.afterEvaluate {
                apiGenerateTasks.forEach { (variantName, apiGenerate) ->
                    val name = variantName.replaceFirstChar { c -> c.titlecase() }
                    apiGenerate.configure {
                        classpath = project.files(
                            project.tasks.named("compile${name}JavaWithJavac", JavaCompile::class.java)
                                .map { it.classpath },
                        )
                    }
                }
            }
        }
    }
}
