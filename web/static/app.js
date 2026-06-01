const HISTORY_LIMIT = 300;
const MAX_ROWS = 200;
const sparklineWindow = 60;

const eventCounts = {
  build: 0,
  load: 0,
  run: 0,
  stop: 0,
  fail: 0,
  trace: 0,
  verify: 0,
  log: 0,
};

const eventsPerSecond = Array(sparklineWindow).fill(0);
let currentSecond = Math.floor(Date.now() / 1000);
let currentSecondCount = 0;

const streamStatus = document.getElementById("streamStatus");
const lastUpdate = document.getElementById("lastUpdate");
const eventTable = document.getElementById("eventTable");

const histogramCtx = document.getElementById("histogram");
const sparklineCtx = document.getElementById("sparkline");

const histogramChart = new Chart(histogramCtx, {
  type: "bar",
  data: {
    labels: Object.keys(eventCounts),
    datasets: [
      {
        label: "events",
        data: Object.values(eventCounts),
        backgroundColor: [
          "#ff6a3d",
          "#2aa9ff",
          "#1ac97b",
          "#f2b84b",
          "#f0515e",
          "#7a63ff",
          "#c78bff",
          "#8c95a8",
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
  return "log";
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
  row.innerHTML = `
    <td>${item.ts ?? "-"}</td>
    <td>${item.module}</td>
    <td>${item.event_type}</td>
    <td>${item.message}</td>
  `;
  eventTable.prepend(row);
  while (eventTable.children.length > MAX_ROWS) {
    eventTable.removeChild(eventTable.lastChild);
  }
}

function applyEvent(item) {
  const type = normalizeEventType(item.event_type);
  eventCounts[type] += 1;
  currentSecondCount += 1;

  histogramChart.data.datasets[0].data = Object.values(eventCounts);
  histogramChart.update("none");
  updateSparkline();
  addRow(item);
  lastUpdate.textContent = new Date().toLocaleTimeString();
}

async function loadHistory() {
  const res = await fetch(`/api/history?lines=${HISTORY_LIMIT}`);
  const data = await res.json();
  data.items.forEach((item) => applyEvent(item));
}

function connectSSE() {
  const source = new EventSource("/events");
  streamStatus.textContent = "connected";

  source.addEventListener("log", (event) => {
    try {
      const item = JSON.parse(event.data);
      applyEvent(item);
    } catch (err) {
      console.error("bad event", err);
    }
  });

  source.onerror = () => {
    streamStatus.textContent = "reconnecting...";
  };
}

loadHistory().then(connectSSE);
