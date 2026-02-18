const statusEl = document.getElementById("status");
const dotEl = document.getElementById("dot");
const statusText = document.getElementById("status-text");
const portText = document.getElementById("port-text");
const portInput = document.getElementById("port");
const connectBtn = document.getElementById("connect-btn");
const retryBtn = document.getElementById("retry-btn");

function updateUI(connected, port) {
  if (connected) {
    statusEl.className = "status connected";
    dotEl.className = "dot on";
    statusText.textContent = "Connected";
    portText.textContent = `Port ${port}`;
    portInput.placeholder = String(port);
  } else {
    statusEl.className = "status disconnected";
    dotEl.className = "dot off";
    statusText.textContent = "Disconnected";
    portText.textContent = "Server not found";
  }
}

function refresh() {
  chrome.runtime.sendMessage({ type: "getStatus" }, (resp) => {
    if (resp) updateUI(resp.connected, resp.port);
  });
}

connectBtn.addEventListener("click", () => {
  const port = parseInt(portInput.value, 10);
  if (!port || port < 1 || port > 65535) return;
  chrome.runtime.sendMessage({ type: "setPort", port }, () => {
    portInput.value = "";
    setTimeout(refresh, 500);
  });
});

retryBtn.addEventListener("click", () => {
  chrome.runtime.sendMessage({ type: "reconnect" }, () => {
    setTimeout(refresh, 1000);
  });
});

refresh();
