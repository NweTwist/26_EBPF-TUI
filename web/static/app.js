const HISTORY_LIMIT = 300;
const MAX_ROWS = 200;
const sparklineWindow = 60;

const EVENT_TYPES = ["build", "load", "run", "stop", "fail"];

const eventCounts = Object.fromEntries(EVENT_TYPES.map((t) => [t, 0]));

const eventsPerSecond = Array(sparklineWindow).fill(0);
let currentSecond = Math.floor(Date.now() / 1000);
let currentSecondCount = 0;

// Dedupe live SSE events (history rows are display-only).
const seenLiveKeys = new Set();
const MAX_SEEN_LIVE_KEYS = 2000;

const streamStatus = document.getElementById("streamStatus");
const lastUpdate = document.getElementById("lastUpdate");
const eventTable = document.getElementById("eventTable");

const histogramCtx = document.getElementById("histogram");
const sparklineCtx = document.getElementById("sparkline");

const histogramChart = new Chart(histogramCtx, {
  type: "bar",
  data: {
    labels: EVENT_TYPES,
    datasets: [
      {
        label: "events",
        data: EVENT_TYPES.map((t) => eventCounts[t]),
        backgroundColor: [
          "#ff6a3d",
          "#2aa9ff",
          "#1ac97b",
          "#f2b84b",
          "#f0515e",
        ],
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
    },
    scales: {
      x: { grid: { display: false } },
      y: { beginAtZero: true },
    },
  },
});

const sparklineChart = new Chart(sparklineCtx, {
  type: "line",
  data: {
    labels: Array.from({ length: sparklineWindow }, (_, i) => `${sparklineWindow - i}s`),
    datasets: [
      {
        label: "eps",
        data: eventsPerSecond,
        borderColor: "#2aa9ff",
        backgroundColor: "rgba(42, 169, 255, 0.2)",
        fill: true,
        tension: 0.35,
        pointRadius: 0,
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
    },
    scales: {
      x: { display: false },
      y: { beginAtZero: true },
    },
  },
});

function normalizeEventType(eventType) {
  if (eventCounts[eventType] !== undefined) {
    return eventType;
  }
  return null;
}

function eventKey(item) {
  if (item.raw) {
    return item.raw;
  }
  return `${item.module}|${item.event_type}|${item.message}`;
}

function updateHistogram() {
  histogramChart.data.datasets[0].data = EVENT_TYPES.map((t) => eventCounts[t]);
  histogramChart.update("none");
}

function updateSparkline() {
  const nowSec = Math.floor(Date.now() / 1000);
  if (nowSec !== currentSecond) {
    const gap = Math.min(nowSec - currentSecond, sparklineWindow);
    for (let i = 0; i < gap - 1; i += 1) {
      eventsPerSecond.push(0);
    }
    eventsPerSecond.push(currentSecondCount);
    while (eventsPerSecond.length > sparklineWindow) {
      eventsPerSecond.shift();
    }
    currentSecondCount = 0;
    currentSecond = nowSec;
    sparklineChart.data.datasets[0].data = eventsPerSecond;
    sparklineChart.update("none");
  }
}

function addRow(item) {
  const row = document.createElement("tr");
  const cells = [
    item.ts ?? "-",
    item.module,
    item.event_type,
    item.message,
  ];
  cells.forEach((value) => {
    const cell = document.createElement("td");
    cell.textContent = value ?? "";
    row.appendChild(cell);
  });
  eventTable.prepend(row);
  while (eventTable.children.length > MAX_ROWS) {
    eventTable.removeChild(eventTable.lastChild);
  }
}

function rememberLiveKey(key) {
  seenLiveKeys.add(key);
  if (seenLiveKeys.size > MAX_SEEN_LIVE_KEYS) {
    const drop = seenLiveKeys.size - MAX_SEEN_LIVE_KEYS;
    const iter = seenLiveKeys.values();
    for (let i = 0; i < drop; i += 1) {
      const next = iter.next();
      if (!next.done) {
        seenLiveKeys.delete(next.value);
      }
    }
  }
}

function applyLiveEvent(item) {
  const type = normalizeEventType(item.event_type);
  if (!type) {
    return;
  }

  const key = eventKey(item);
  if (seenLiveKeys.has(key)) {
    return;
  }
  rememberLiveKey(key);

  eventCounts[type] += 1;
  currentSecondCount += 1;

  updateHistogram();
  updateSparkline();
  addRow(item);
  lastUpdate.textContent = new Date().toLocaleTimeString();
}

async function loadStats() {
  const res = await fetch("/api/stats");
  const data = await res.json();
  for (const type of EVENT_TYPES) {
    eventCounts[type] = data.counts?.[type] ?? 0;
  }
  updateHistogram();
}

async function loadHistory() {
  const res = await fetch(`/api/history?lines=${HISTORY_LIMIT}`);
  const data = await res.json();
  const items = data.items ?? [];
  for (let i = items.length - 1; i >= 0; i -= 1) {
    addRow(items[i]);
  }
  if (items.length > 0) {
    const last = items[items.length - 1];
    lastUpdate.textContent = last.ts ?? new Date().toLocaleTimeString();
  }
}

function resetUiState() {
  for (const type of EVENT_TYPES) {
    eventCounts[type] = 0;
  }
  seenLiveKeys.clear();
  currentSecondCount = 0;
  eventsPerSecond.fill(0);
  currentSecond = Math.floor(Date.now() / 1000);
  updateHistogram();
  sparklineChart.data.datasets[0].data = eventsPerSecond;
  sparklineChart.update("none");
  eventTable.replaceChildren();
  lastUpdate.textContent = "-";
}

function connectSSE() {
  const source = new EventSource("/events");
  streamStatus.textContent = "connected";

  source.addEventListener("log", (event) => {
    try {
      const item = JSON.parse(event.data);
      applyLiveEvent(item);
    } catch (err) {
      console.error("bad event", err);
    }
  });

  source.addEventListener("reset", () => {
    resetUiState();
  });

  source.onerror = () => {
    streamStatus.textContent = "reconnecting...";
  };
}

async function init() {
  await loadStats();
  await loadHistory();
  connectSSE();
}

init();
