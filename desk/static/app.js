const summary = document.querySelector("#summary");
const positions = document.querySelector("#positions");
const orders = document.querySelector("#orders");
const fills = document.querySelector("#fills");
const form = document.querySelector("#ticket");
const message = document.querySelector("#ticket-message");
const submit = form.querySelector("button");
const strategyInput = form.elements.strategy;
const strategyList = document.querySelector("#strategies");
const bookStrategy = document.querySelector("#book-strategy");
const storedStrategy = localStorage.getItem("desk.strategy");
if (storedStrategy) strategyInput.value = storedStrategy;

let selectedBook = localStorage.getItem("desk.bookStrategy") || "";
let latest = { book: [], orders: [], fills: [] };

bookStrategy.addEventListener("change", () => {
  selectedBook = bookStrategy.value;
  localStorage.setItem("desk.bookStrategy", selectedBook);
  paint();
});

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const data = new FormData(form);
  submit.disabled = true;
  message.textContent = "Forwarding…";
  message.dataset.tone = "";
  try {
    const response = await fetch("/api/orders", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        strategy: String(data.get("strategy")).trim(),
        instrument_id: data.get("instrument_id").trim(),
        side: data.get("side"),
        quantity: Number(data.get("quantity")),
        price: String(data.get("price")).trim(),
      }),
    });
    const payload = await response.json();
    if (!response.ok) {
      message.textContent = payload.error || "The desk refused the order.";
      message.dataset.tone = "bad";
    } else {
      const order = payload.order;
      localStorage.setItem("desk.strategy", order.strategy);
      selectedBook = order.strategy;
      localStorage.setItem("desk.bookStrategy", selectedBook);
      message.textContent = `Forwarded ${order.side} ${order.quantity} ${order.instrument_id} at ${order.price} into ${order.strategy} through the ${order.gateway} gateway.`;
      message.dataset.tone = "good";
    }
    await refresh();
  } catch (error) {
    message.textContent = "The desk could not be reached.";
    message.dataset.tone = "bad";
  } finally {
    submit.disabled = false;
  }
});

async function refresh() {
  const [book, orderRows, fillRows] = await Promise.all([
    getJson("/api/positions"),
    getJson("/api/orders"),
    getJson("/api/fills"),
  ]);
  latest = { book, orders: orderRows, fills: fillRows };
  fillStrategyPicker(book);
  paint();
  rememberStrategies(book, orderRows);
}

function paint() {
  const rows = latest.book.filter((row) => row.strategy === selectedBook);
  renderSummary(rows);
  renderPositions(rows);
  renderOrders(latest.orders);
  renderFills(latest.fills);
}

function fillStrategyPicker(book) {
  const names = [...new Set(book.filter((row) => row.quantity !== 0).map((row) => row.strategy))].sort();
  if (names.length === 0) {
    bookStrategy.disabled = true;
    bookStrategy.innerHTML = `<option value="">No open strategies</option>`;
    selectedBook = "";
    return;
  }
  bookStrategy.disabled = false;
  bookStrategy.innerHTML = names.map((name) => `<option value="${escapeHtml(name)}">${escapeHtml(name)}</option>`).join("");
  if (!names.includes(selectedBook)) selectedBook = names[0];
  bookStrategy.value = selectedBook;
  localStorage.setItem("desk.bookStrategy", selectedBook);
}

async function getJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(await response.text());
  }
  return response.json();
}

function renderSummary(rows) {
  const open = rows.filter((row) => row.quantity !== 0);
  const net = open.reduce((total, row) => total + row.quantity, 0);
  const realized = rows.reduce((total, row) => total + Number(row.realized_pnl), 0);
  summary.innerHTML = `
    <article><span class="eyebrow">Open names</span><strong>${open.length}</strong></article>
    <article><span class="eyebrow">Net quantity</span><strong>${net}</strong></article>
    <article><span class="eyebrow">Realized PnL</span><strong class="${realized < 0 ? "short" : "long"}">${formatNumber(realized)}</strong></article>
  `;
}

function renderPositions(rows) {
  const open = rows.filter((row) => row.quantity !== 0);
  if (!selectedBook) {
    positions.innerHTML = `<p class="empty">No open position. A fill from the gateway will appear here.</p>`;
    return;
  }
  if (open.length === 0) {
    positions.innerHTML = `<p class="empty">No open position in ${escapeHtml(selectedBook)}.</p>`;
    return;
  }
  positions.innerHTML = table(
    ["Instrument", "Quantity", "Average", "Realized"],
    open.map((row) => [
      mono(row.instrument_id),
      signed(row.quantity),
      row.avg_price,
      row.realized_pnl,
    ]),
  );
}

function renderOrders(rows) {
  if (rows.length === 0) {
    orders.innerHTML = `<p class="empty">No orders yet.</p>`;
    return;
  }
  orders.innerHTML = table(
    ["Time", "Strategy", "Instrument", "Qty", "Price", "Status"],
    rows.map((row) => [
      time(row.created_at),
      escapeHtml(row.strategy),
      mono(row.instrument_id),
      signed(directed(row.side, row.quantity)),
      row.price,
      `${row.status}${row.reason ? `<span class="reason">${escapeHtml(row.reason)}</span>` : `<span class="eyebrow">${escapeHtml(row.gateway)}</span>`}`,
    ]),
  );
}

function renderFills(rows) {
  if (rows.length === 0) {
    fills.innerHTML = `<p class="empty">No fills yet.</p>`;
    return;
  }
  fills.innerHTML = table(
    ["Time", "Strategy", "Instrument", "Qty", "Price"],
    rows.map((row) => [
      time(row.filled_at),
      escapeHtml(row.strategy),
      mono(row.instrument_id),
      signed(directed(row.side, row.quantity)),
      row.price,
    ]),
  );
}

function rememberStrategies(book, orderRows) {
  const names = new Set([
    ...book.map((row) => row.strategy),
    ...orderRows.map((row) => row.strategy),
  ]);
  strategyList.innerHTML = [...names]
    .sort()
    .map((name) => `<option value="${escapeHtml(name)}"></option>`)
    .join("");
}

function table(headers, rows) {
  const head = headers.map((header) => `<th>${header}</th>`).join("");
  const body = rows
    .map((row) => `<tr>${row.map((cell) => `<td>${cell}</td>`).join("")}</tr>`)
    .join("");
  return `<table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
}

function directed(side, quantity) {
  return side === "SELL" ? -quantity : quantity;
}

function signed(quantity) {
  const klass = quantity > 0 ? "long" : "short";
  const text = quantity > 0 ? `+${quantity}` : String(quantity);
  return `<span class="${klass}">${text}</span>`;
}

function mono(value) {
  return `<span class="mono">${escapeHtml(value)}</span>`;
}

function time(value) {
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    day: "2-digit",
    month: "short",
  }).format(new Date(value));
}

function formatNumber(value) {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 4 }).format(value);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

refresh().catch(() => {
  positions.innerHTML = `<p class="empty">The position book could not be loaded.</p>`;
});
