// Graba demo.html?scene=story (animaciones reales) con el screencast de Chrome y la convierte en docs/img/demo.gif.
// Uso, con `npm run dev` arrancado:
//   npm i --no-save puppeteer-core gifenc pngjs
//   node scripts/record-gif.mjs docs/img/demo.gif
import puppeteer from "puppeteer-core";
import { PNG } from "pngjs";
import gifenc from "gifenc";
import fs from "node:fs";
const { GIFEncoder, quantize, applyPalette } = gifenc;

const OUT = process.argv[2] ?? "docs/img/demo.gif";
const SCALE = Number(process.argv[3] ?? 1);
const browser = await puppeteer.launch({
  executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
  headless: "new",
  args: ["--disable-gpu", "--hide-scrollbars"],
});
const page = await browser.newPage();
await page.setViewport({ width: 480, height: 300, deviceScaleFactor: SCALE });
const cdp = await page.createCDPSession();
const frames = [];
cdp.on("Page.screencastFrame", async (f) => {
  frames.push({ data: f.data, t: f.metadata.timestamp });
  await cdp.send("Page.screencastFrameAck", { sessionId: f.sessionId }).catch(() => {});
});
await page.goto("http://localhost:1420/demo.html?scene=story", { waitUntil: "networkidle0" });
await cdp.send("Page.startScreencast", { format: "png", everyNthFrame: 1 });
await page.waitForFunction(() => document.body.dataset.done === "1", { timeout: 60000, polling: 200 });
await cdp.send("Page.stopScreencast");
await browser.close();
console.log("fotogramas:", frames.length);

// Paleta común a partir de una muestra de fotogramas (evita parpadeos de color).
// Como mucho un fotograma cada 50 ms y sin repetir fotogramas idénticos.
const decoded = [];
for (const f of frames) {
  const prev = decoded[decoded.length - 1];
  if (prev && f.t - prev.t < 0.05) continue;
  const png = PNG.sync.read(Buffer.from(f.data, "base64"));
  if (prev && Buffer.compare(prev.png.data, png.data) === 0) continue;
  decoded.push({ ...f, png });
}
console.log("tras filtrar:", decoded.length);
const { width, height } = decoded[0].png;
const sample = decoded.filter((_, i) => i % Math.max(1, Math.floor(decoded.length / 24)) === 0);
const pool = new Uint8Array(sample.length * width * height * 4);
sample.forEach((f, i) => pool.set(f.png.data, i * width * height * 4));
const palette = quantize(pool, 256);
const gif = GIFEncoder();
for (let i = 0; i < decoded.length; i++) {
  const next = decoded[i + 1]?.t ?? decoded[i].t + 1.2;
  const delay = Math.max(20, Math.round((next - decoded[i].t) * 1000));
  gif.writeFrame(applyPalette(decoded[i].png.data, palette), width, height, { palette: i === 0 ? palette : undefined, delay });
}
gif.finish();
fs.writeFileSync(OUT, gif.bytes());
console.log(`${OUT}: ${width}x${height}, ${(gif.bytes().length / 1024 / 1024).toFixed(2)} MB`);

