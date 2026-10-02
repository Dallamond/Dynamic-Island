// statusLine de Claude Code: imprime una línea corta y reenvía el JSON a la Dynamic Island.
// Si la isla está cerrada, el envío falla en silencio y la línea se muestra igual.
const PORT = 47823;

let input = "";
for await (const chunk of process.stdin) input += chunk;

let d = {};
try {
  d = JSON.parse(input);
} catch {}

const parts = [];
if (d.model?.display_name) parts.push(d.model.display_name);
const ctx = d.context_window?.used_percentage;
if (ctx != null) parts.push(`ctx ${Math.round(ctx)}%`);
const cost = d.cost?.total_cost_usd;
if (cost != null) parts.push(`$${cost.toFixed(2)}`);
const h5 = d.rate_limits?.five_hour?.used_percentage;
if (h5 != null) parts.push(`5h ${Math.round(h5)}%`);
const w = d.rate_limits?.seven_day?.used_percentage;
if (w != null) parts.push(`7d ${Math.round(w)}%`);
process.stdout.write(parts.join(" · "));

try {
  await fetch(`http://127.0.0.1:${PORT}/status`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: input,
    signal: AbortSignal.timeout(400),
  });
} catch {}
