const summary = document.querySelector("#summary");
const positions = document.querySelector("#positions");
const orders = document.querySelector("#orders");
const fills = document.querySelector("#fills");
const form = document.querySelector("#ticket");
const message = document.querySelector("#ticket-message");
const submit = form.querySelector("button");
const strategyInput = form.elements.strategy;
const strategyList = document.querySelector("#strategies");
const storedStrategy = localStorage.getItem("desk.strategy");
if (storedStrategy) strategyInput.value = storedStrategy;

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
  renderSummary(book);
  renderPositions(book);
  renderOrders(orderRows);
  renderFills(fillRows);
  rememberStrategies(book, orderRows);
}

async function getJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(await response.text());
  }
  return response.json();
}

function renderSummary(book) {
  const open = book.filter((row) => row.quantity !== 0);
  const strategies = new Set(open.map((row) => row.strategy)).size;
  const realized = book.reduce((total, row) => total + Number(row.realized_pnl), 0);
  summary.innerHTML = `
    <article><span class="eyebrow">Strategies</span><strong>${strategies}</strong></article>
    <article><span class="eyebrow">Open names</span><strong>${open.length}</strong></article>
    <article><span class="eyebrow">Realized PnL</span><strong class="${realized < 0 ? "short" : "long"}">${formatNumber(realized)}</strong></article>
  `;
}

function renderPositions(book) {
  const open = book.filter((row) => row.quantity !== 0);
  if (open.length === 0) {
    positions.innerHTML = `<p class="empty">No open position. A fill from the gateway will appear here.</p>`;
    return;
  }
  const groups = new Map();
  for (const row of open) {
    if (!groups.has(row.strategy)) groups.set(row.strategy, []);
    groups.get(row.strategy).push(row);
  }
  positions.innerHTML = [...groups.entries()]
    .map(
      ([strategy, rows]) => `
        <section class="strategy-group">
          <h3>${escapeHtml(strategy)}</h3>
          ${table(
            ["Instrument", "Side", "Quantity", "Average", "Realized"],
            rows.map((row) => [
              mono(row.instrument_id),
              sideLabel(row.quantity),
              signed(row.quantity),
              row.avg_price,
              row.realized_pnl,
            ]),
          )}
        </section>`,
    )
    .join("");
}

function renderOrders(rows) {
  if (rows.length === 0) {
    orders.innerHTML = `<p class="empty">No orders yet.</p>`;
    return;
  }
  orders.innerHTML = table(
    ["Time", "Strategy", "Instrument", "Side", "Qty", "Price", "Status"],
    rows.map((row) => [
      time(row.created_at),
      escapeHtml(row.strategy),
      mono(row.instrument_id),
      sideText(row.side),
      String(row.quantity),
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
    ["Time", "Strategy", "Instrument", "Side", "Qty", "Price"],
    rows.map((row) => [
      time(row.filled_at),
      escapeHtml(row.strategy),
      mono(row.instrument_id),
      sideText(row.side),
      String(row.quantity),
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

function sideLabel(quantity) {
  if (quantity > 0) return `<span class="long">Long</span>`;
  if (quantity < 0) return `<span class="short">Short</span>`;
  return "Flat";
}

function sideText(side) {
  return side === "BUY" ? `<span class="long">Buy</span>` : `<span class="short">Sell</span>`;
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
