/* Any copyright is dedicated to the Public Domain.
   https://creativecommons.org/publicdomain/zero/1.0/ */

"use strict";

const { AgentUI } = ChromeUtils.importESModule(
  "moz-src:///browser/components/aiwindow/ui/modules/AgentUI.sys.mjs"
);

const { MonitorAgent } = ChromeUtils.importESModule(
  "moz-src:///browser/components/aiwindow/models/agents/MonitorAgent.sys.mjs"
);

const PREF_AGENT_ENABLED = "browser.smartwindow.agent.enabled";

function makeConversationStub() {
  let resolveSeeded;
  const seeded = new Promise(resolve => {
    resolveSeeded = resolve;
  });
  const assistantMessages = [];
  const conversation = {
    addUserMessage: () => ({}),
    emit: () => {},
    addAssistantMessage: (_type, body) => assistantMessages.push(body),
    addUIToolToCurrentMessage: (_id, data) => resolveSeeded(data),
  };
  return { conversation, seeded, assistantMessages };
}

add_task(async function test_monitor_command_prefills_condition() {
  await SpecialPowers.pushPrefEnv({ set: [[PREF_AGENT_ENABLED, true]] });
  await MonitorAgent._resetForTesting();

  try {
    const { conversation, seeded, assistantMessages } = makeConversationStub();
    const handled = AgentUI.tryHandleCommand({
      command: "monitor",
      value: "/monitor the price drops below $200",
      contextPageUrl: "https://example.com/product",
      conversation,
    });
    Assert.ok(handled, "The /monitor command is handled");

    const { properties } = await seeded;
    Assert.equal(
      properties.agent.condition,
      "the price drops below $200",
      "The monitor card is seeded with the text typed after /monitor"
    );
    Assert.ok(
      assistantMessages.some(body => body?.includes("watch this page")),
      "The localized monitor-setup message is shown"
    );
  } finally {
    await MonitorAgent._resetForTesting();
    await SpecialPowers.popPrefEnv();
  }
});

add_task(async function test_bare_monitor_command_seeds_empty_condition() {
  await SpecialPowers.pushPrefEnv({ set: [[PREF_AGENT_ENABLED, true]] });
  await MonitorAgent._resetForTesting();

  try {
    const { conversation, seeded } = makeConversationStub();
    AgentUI.tryHandleCommand({
      value: "/monitor",
      contextPageUrl: "https://example.com/product",
      conversation,
    });

    const { properties } = await seeded;
    Assert.equal(
      properties.agent.condition,
      "",
      "A bare /monitor command seeds an empty condition"
    );
  } finally {
    await MonitorAgent._resetForTesting();
    await SpecialPowers.popPrefEnv();
  }
});

add_task(async function test_create_monitor_localizes_schedule_summary() {
  await SpecialPowers.pushPrefEnv({ set: [[PREF_AGENT_ENABLED, true]] });
  await MonitorAgent._resetForTesting();

  try {
    const { conversation } = makeConversationStub();
    const message = { content: {}, toolUIData: { properties: { agent: {} } } };
    const updateData = {
      monitorName: "r/Watchexchange",
      condition: "new posts",
      watchUrls: ["https://example.com/watches"],
      schedule: { frequency: "daily", time: "09:00", weekday: "1" },
    };

    const created = await AgentUI.handleCreateMonitor({
      message,
      updateData,
      conversation,
    });
    Assert.ok(created, "The monitor is created");

    Assert.equal(
      message.content.l10nId,
      "smartwindow-agent-monitor-watching",
      "The watching message renders from its l10n id"
    );

    const { schedule } = message.content.l10nArgs;

    Assert.ok(
      schedule.startsWith("daily at") && /\d/.test(schedule),
      `The schedule arg is a localized cadence string, got: "${schedule}"`
    );
    Assert.ok(
      !schedule.includes("DATETIME") && !schedule.includes("[object"),
      "The schedule arg is fully resolved"
    );
  } finally {
    await MonitorAgent._resetForTesting();
    await SpecialPowers.popPrefEnv();
  }
});
