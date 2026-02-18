// Content script for on-demand page content extraction.
// Injected by the background service worker when getContent
// needs reader-mode or markdown extraction from a page.

(() => {
  // Simple text extraction - the main extraction logic runs
  // via chrome.scripting.executeScript in handlers.js.
  // This content script is available for future enhancements
  // like Readability-based article extraction.
})();
