/**
 * Command handlers that bridge Chrome extension APIs to Cocommand.
 */

export async function getTabs() {
  const tabs = await chrome.tabs.query({});
  return tabs.map((tab) => ({
    id: tab.id,
    url: tab.url,
    title: tab.title,
    active: tab.active,
    favicon: tab.favIconUrl || null,
  }));
}

export async function getActiveTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab) return null;
  return {
    id: tab.id,
    url: tab.url,
    title: tab.title,
    active: tab.active,
    favicon: tab.favIconUrl || null,
  };
}

export async function getContent(params) {
  const format = params.format || "text";
  const cssSelector = params.cssSelector || null;

  // Determine which tab to use.
  let tabId = params.tabId;
  if (!tabId) {
    const [active] = await chrome.tabs.query({
      active: true,
      currentWindow: true,
    });
    if (!active) throw new Error("No active tab found");
    tabId = active.id;
  }

  const results = await chrome.scripting.executeScript({
    target: { tabId },
    args: [format, cssSelector],
    func: (fmt, selector) => {
      let root = document;
      if (selector) {
        const el = document.querySelector(selector);
        if (!el) return { error: `No element matching '${selector}'` };
        root = el;
      }

      if (fmt === "html") {
        return { content: root === document ? document.documentElement.outerHTML : root.outerHTML };
      }
      // Default to text.
      return { content: root === document ? document.body.innerText : root.innerText };
    },
  });

  const result = results?.[0]?.result;
  if (!result) throw new Error("Script execution returned no result");
  if (result.error) throw new Error(result.error);
  return result;
}
