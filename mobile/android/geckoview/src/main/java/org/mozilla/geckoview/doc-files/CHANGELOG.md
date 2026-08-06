---
layout: default
title: API Changelog
description: GeckoView API Changelog.
nav_exclude: true
exclude: true
---

{% capture javadoc_uri %}{{ site.url }}{{ site.baseurl}}/javadoc/mozilla-central/org/mozilla/geckoview{% endcapture %}
{% capture bugzilla %}https://bugzilla.mozilla.org/show_bug.cgi?id={% endcapture %}

# GeckoView API Changelog.

⚠️  breaking change and deprecation notices

## v155
- Added the [`IPProxyException.ERROR_CATASTROPHIC`][155.1] and [`IPProxyException.ERROR_VPN_UNAVAILABLE`][155.2]
  error codes for the [`IPProtectioController.activate`][155.3].

[155.1]: {{javadoc_uri}}/IPProtectionController.IPProxyException.html#ERROR_CATASTROPHIC
[155.2]: {{javadoc_uri}}/IPProtectionController.IPProxyException.html#ERROR_VPN_UNAVAILABLE
[155.3]: {{javadoc_uri}}/IPProtectionController.html#activate(boolean,boolean,java.lang.String)

## v155
- Added [WebRequestError.ERROR_LOCAL_NETWORK_ACCESS_DENIED] to indicate that a load failed because the user denied the local network access permission on Android 17+.

## v154
- Added [`Autofill.Node.getDatalist`][154.1] to expose predefined values by [`datalist`][154.2] elements for input fields.
- Added experimental [`ContentPermission.notifyShown`][154.3] so embedders can signal that a permission prompt UI has been displayed to the user, enabling per-prompt telemetry on the Gecko side (e.g. for local network access). ([bug 2009145]({{bugzilla}}2009145))
- Added [`IPProtectionController.refreshUsage`][154.4] to request a refresh of proxy usage information, delivered asynchronously via [`Delegate#onUsageChanged`][154.5].
  ([bug 2042799]({{bugzilla}}2042799))
- Added [`IPProtectionController.getCountryList`][154.6] and [`IPProtectionController.Country`][154.7] to request the list of countries available in the proxy serverlist, delivered asynchronously via [`Delegate#onCountryListChanged`][154.8] whenever the list changes.
- Added [`IPProtectionController.activate(boolean, boolean, String)`][154.9] to activate the proxy with explicit user-action, private-browsing, and country options.
- ⚠️ Made [`ScrollPositionUpdate`][154.10] immutable: its fields are now `final` and instances are constructed via `ScrollPositionUpdate(float, float, float, int)` instead of the previous no-argument constructor with mutable fields. ([bug 1994863]({{bugzilla}}1994863))
- Added [`GeckoSession.getBrokenSiteReport`][154.11] that returns a `GeckoResult<JSONObject>` containing information for a broken site report. ([bug 2049050]({{bugzilla}}2049050)).
- Changed [`GeckoSession.setHistoryDelegate`][154.12], [`setContentBlockingDelegate`][154.13], [`setMediaDelegate`][154.14], [`setMediaSessionDelegate`][154.15], [`setTranslationsSessionDelegate`][154.16], [`setPrintDelegate`][154.17], and [`setExperimentDelegate`][154.18] from `@AnyThread` to `@UiThread`, reflecting that they must be called on the UI thread.
- Added [`MediaSession.notifySystemAudioFocusChange`][154.19] so embedders can route a system audio-focus change to the tab's W3C Audio Session interrupt, suspending and resuming the tab's audible media elements, Web Audio, and Web Speech. ([bug 2048732]({{bugzilla}}2048732))
- Added [`GeckoSession.sendGleanBrokenSiteReport`][154.20] which sends a broken site report using Glean. ([bug 2054543]({{bugzilla}}2054543)).
- Added [`GeckoSession.HistoryDelegate.hasVisitedHostSince`][154.21] so embedders can report whether a host was visited within a time window, used to derive first-daily-load pageload telemetry. ([bug 2058980]({{bugzilla}}2058980))
- Added experimental [`GeckoRuntimeSettings.setIpProtectionAuthProvider`][154.22] and [`getIpProtectionAuthProvider`][154.23] to select the IP Protection authentication provider (`"fxa"` or `"gpi"`) on Android. ([bug 2054901]({{bugzilla}}2054901))
- ⚠️ Removed the Cookie Banner Handling API. The underlying Gecko feature no longer exists and
  there is no replacement. The following members were removed:
  `ContentBlocking.CookieBannerMode` and `ContentBlocking.CBCookieBannerMode`;
  `ContentBlocking.Settings.setCookieBannerMode`, `getCookieBannerMode`,
  `setCookieBannerModePrivateBrowsing`, `getCookieBannerModePrivateBrowsing`,
  `setCookieBannerDetectOnlyMode`, `getCookieBannerDetectOnlyMode`,
  `setCookieBannerGlobalRulesEnabled`, `getCookieBannerGlobalRulesEnabled`,
  `setCookieBannerGlobalRulesSubFramesEnabled` and
  `getCookieBannerGlobalRulesSubFramesEnabled`;
  `ContentBlocking.Settings.Builder.cookieBannerHandlingMode`,
[api-version]: 8746a786b9e9fd08dd61321d2973efc43d1ffefa
