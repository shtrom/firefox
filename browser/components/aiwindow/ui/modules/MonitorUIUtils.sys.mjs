/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

const lazy = {};
ChromeUtils.defineESModuleGetters(lazy, {
  MonitorAgent:
    "moz-src:///browser/components/aiwindow/models/agents/MonitorAgent.sys.mjs",
});

const localization = new Localization(
  ["preview/aiWindow.ftl", "branding/brand.ftl"],
  true
);

/**
 * Shared utilities for monitor UI operations
 */
export const MonitorUIUtils = {
  /**
   * Show a confirmation prompt before deleting a monitor and delete if confirmed.
   *
   * @param {BrowsingContext} browsingContext - The browsing context for the prompt
   * @param {string} monitorId - The ID of the monitor to delete
   * @param {boolean} skipConfirmation - Skip confirmation dialog (for tests)
   * @returns {Promise<{success: boolean, deleted: boolean, cancelled: boolean}>}
   */
  async deleteMonitorWithConfirmation(
    browsingContext,
    monitorId,
    skipConfirmation = false
  ) {
    try {
      let confirmed = skipConfirmation;

      if (!skipConfirmation) {
        // Localize the prompt strings
        const [title, message, deleteButton] = await localization.formatValues([
          { id: "ai-tasks-alert-delete-confirmation-title" },
          { id: "ai-tasks-alert-delete-confirmation-message" },
          { id: "ai-tasks-alert-delete-confirm-button" },
        ]);

        // Set up the button flags for the prompt
        const flags =
          (Ci.nsIPromptService.BUTTON_TITLE_IS_STRING *
            Ci.nsIPromptService.BUTTON_POS_0) |
          (Ci.nsIPromptService.BUTTON_TITLE_CANCEL *
            Ci.nsIPromptService.BUTTON_POS_1) |
          Ci.nsIPromptService.BUTTON_POS_1_DEFAULT;

        // Show the confirmation prompt
        const result = await Services.prompt.asyncConfirmEx(
          browsingContext,
          Ci.nsIPrompt.MODAL_TYPE_INTERNAL_WINDOW,
          title,
          message,
          flags,
          deleteButton,
          null,
          null,
          null,
          false,
          { useTitle: true }
        );

        // Check if user clicked Delete (button 0)
        confirmed = result.get("buttonNumClicked") === 0;
      }

      if (!confirmed) {
        return {
          success: true,
          deleted: false,
          cancelled: true,
        };
      }

      // User confirmed, proceed with deletion
      const deleted = await lazy.MonitorAgent.deleteMonitor(monitorId);
      return {
        success: true,
        deleted,
        cancelled: false,
      };
    } catch (error) {
      console.error("Failed to delete monitor:", error);
      return {
        success: false,
        deleted: false,
        cancelled: false,
        error: error.message,
      };
    }
  },
};
