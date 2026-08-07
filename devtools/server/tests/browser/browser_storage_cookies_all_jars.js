/* Any copyright is dedicated to the Public Domain.
   http://creativecommons.org/publicdomain/zero/1.0/ */

"use strict";

// In an everything-scope session (the Browser Toolbox shape) a single cookies
// storage actor serves the whole browser, and one host row can be backed by
// several cookie jars at once. Check that cookie changes in every jar backing
// the host reach the session, and that a change in a jar no live context uses
// does not.

const HOST = "example.com";
const ORIGIN = `https://${HOST}`;
const TEST_URL = `${ORIGIN}/document-builder.sjs?html=jars`;
const SUFFIX = `-${Date.now()}`;

const NORMAL = { name: `normal${SUFFIX}`, originAttributes: {} };
const PRIVATE = {
  name: `private${SUFFIX}`,
  originAttributes: { privateBrowsingId: 1 },
};
const CONTAINER = {
  name: `container${SUFFIX}`,
  originAttributes: { userContextId: 2 },
};
// No live context uses this jar, so its changes must not be forwarded.
const ORPHAN = {
  name: `orphan${SUFFIX}`,
  originAttributes: { userContextId: 9 },
};

registerCleanupFunction(() => {
  Services.cookies.removeAll();
});

add_task(async function () {
  await pushPref("devtools.browsertoolbox.scope", "everything");
  // Required by CommandsFactory.forMainProcess
  await pushPref("devtools.chrome.enabled", true);

  const normalTab = await addTab(TEST_URL);
  const privateWindow = await BrowserTestUtils.openNewBrowserWindow({
    private: true,
  });
  await addTab(TEST_URL, { window: privateWindow });
  const containerTab = await addTab(TEST_URL, { userContextId: 2 });

  const commands = await CommandsFactory.forMainProcess();
  await commands.targetCommand.startListening();
  const { resourceCommand } = commands;

  const resources = [];
  await resourceCommand.watchResources([resourceCommand.TYPES.COOKIE], {
    onAvailable: newResources => resources.push(...newResources),
  });
  is(resources.length, 1, "The whole browser has a single cookies resource");
  const cookiesResource = resources[0];
  ok(ORIGIN in cookiesResource.hosts, `${ORIGIN} is a host of the session`);

  // Updates close to each other coalesce into one batched event, so collect
  // the reported keys and wait for the expected set instead of awaiting an
  // event per write.
  const seen = new Set();
  cookiesResource.on("single-store-update", ({ added, changed }) => {
    for (const bucket of [added, changed]) {
      for (const key of bucket?.cookies?.[ORIGIN] ?? []) {
        seen.add(key);
      }
    }
  });

  for (const { name, originAttributes } of [
    NORMAL,
    PRIVATE,
    CONTAINER,
    ORPHAN,
  ]) {
    addCookie(name, originAttributes);
  }

  await waitFor(
    () => [NORMAL, PRIVATE, CONTAINER].every(c => seen.has(uniqueKey(c.name))),
    "Waiting for the changes from all three jars"
  );
  ok(
    true,
    "Changes from the normal, private and container jars all reached the session"
  );

  // Longer than the update batching delay
  await wait(500);
  ok(
    !seen.has(uniqueKey(ORPHAN.name)),
    "A change in a jar no live context uses was not forwarded"
  );

  await BrowserTestUtils.closeWindow(privateWindow);
  BrowserTestUtils.removeTab(containerTab);
  BrowserTestUtils.removeTab(normalTab);
  commands.targetCommand.destroy();
  await commands.destroy();
});

function uniqueKey(name) {
  return `${name}${SEPARATOR_GUID}${HOST}${SEPARATOR_GUID}/${SEPARATOR_GUID}`;
}

function addCookie(name, originAttributes) {
  const validation = Services.cookies.add(
    HOST,
    "/",
    name,
    "value",
    false, // isSecure
    false, // isHttpOnly
    false, // isSession
    Date.now() + 1000 * 60 * 60, // expiry
    originAttributes,
    Ci.nsICookie.SAMESITE_UNSET,
    Ci.nsICookie.SCHEME_HTTPS
  );
  is(validation.result, Ci.nsICookieValidation.eOK, `Cookie ${name} was added`);
}
