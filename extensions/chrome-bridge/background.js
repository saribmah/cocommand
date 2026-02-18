import { getTabs, getActiveTab, getContent } from "./handlers.js";

const MAX_BACKOFF_MS = 30_000;

let ws = null;
let backoff = 1000;
let connectedPort = null;

const handlers = {
  getTabs,
  getActiveTab,
  getContent,
};

/** Read user-configured port from storage. */
async function getSavedPort() {
  try {
    const { port } = await chrome.storage.local.get("port");
    return port ? Number(port) : null;
  } catch {
    return null;
  }
}

/** Probe a port for the Cocommand health endpoint. */
async function probePort(port) {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 500);
    const resp = await fetch(`http://127.0.0.1:${port}/health`, {
      signal: controller.signal,
    });
    clearTimeout(timer);
    if (resp.ok) {
      const text = await resp.text();
      if (text === "ok") return port;
    }
  } catch {
    // Not reachable.
  }
  return null;
}

function connect(port) {
  if (ws) {
    ws.close();
    ws = null;
  }

  console.log(`[cocommand] connecting to ws://127.0.0.1:${port}/browser/ws`);
  const socket = new WebSocket(`ws://127.0.0.1:${port}/browser/ws`);
  ws = socket;

  socket.addEventListener("open", () => {
    if (ws !== socket) return;
    console.log(`[cocommand] connected on port ${port}`);
    connectedPort = port;
    backoff = 1000;
    updateIcon(true);
  });

  socket.addEventListener("message", async (event) => {
    if (ws !== socket) return;

    let msg;
    try {
      msg = JSON.parse(event.data);
    } catch {
      console.warn("[cocommand] invalid message:", event.data);
      return;
    }

    const { id, method, params } = msg;
    if (!id || !method) return;

    const handler = handlers[method];
    if (!handler) {
      socket.send(JSON.stringify({ id, error: `unknown method: ${method}` }));
      return;
    }

    try {
      const result = await handler(params || {});
      socket.send(JSON.stringify({ id, result }));
    } catch (err) {
      socket.send(JSON.stringify({ id, error: err.message || String(err) }));
    }
  });

  socket.addEventListener("close", () => {
    if (ws !== socket) return;
    console.log("[cocommand] disconnected, reconnecting...");
    ws = null;
    connectedPort = null;
    updateIcon(false);
    scheduleReconnect();
  });

  socket.addEventListener("error", () => {
    // close event will fire after error, so reconnect is handled there.
  });
}

function scheduleReconnect() {
  const delay = Math.min(backoff, MAX_BACKOFF_MS);
  backoff = Math.min(backoff * 2, MAX_BACKOFF_MS);
  console.log(`[cocommand] retrying in ${delay}ms...`);
  setTimeout(tryConnect, delay);
}

async function tryConnect() {
  const saved = await getSavedPort();
  if (saved) {
    const verified = await probePort(saved);
    if (verified) {
      connect(verified);
      return;
    }
  }
  console.log("[cocommand] no port configured or server not reachable. Set port via the popup.");
  scheduleReconnect();
}

function updateIcon(connected) {
  const color = connected ? "#22c55e" : "#6b7280";
  chrome.action.setBadgeText({ text: connected ? "ON" : "" });
  chrome.action.setBadgeBackgroundColor({ color });
}

// Message handler for popup communication.
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg.type === "getStatus") {
    sendResponse({
      connected: ws !== null && ws.readyState === WebSocket.OPEN,
      port: connectedPort,
    });
    return false;
  }
  if (msg.type === "setPort") {
    chrome.storage.local.set({ port: msg.port });
    if (ws) {
      ws.close();
      ws = null;
    }
    backoff = 1000;
    tryConnect();
    sendResponse({ ok: true });
    return false;
  }
  if (msg.type === "reconnect") {
    if (ws) {
      ws.close();
      ws = null;
    }
    backoff = 1000;
    tryConnect();
    sendResponse({ ok: true });
    return false;
  }
  return false;
});

// Start connection on service worker load.
tryConnect();
