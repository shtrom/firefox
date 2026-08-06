/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Loaded but not applied, so that both subprojects resolve Spotless from one classloader. Applying
// it independently in sibling projects gives each its own copy of Spotless' shared build service,
// which Gradle rejects.
plugins {
    alias(libs.plugins.spotless) apply false
}
