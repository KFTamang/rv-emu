import express from "express";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const app = express();
const PORT = process.env.PORT ? Number(process.env.PORT) : 3001;

const ROOT_DIR = path.join(__dirname, "..");
const PUBLIC_DIR = path.join(ROOT_DIR, "public");

const DISASM_PATH = process.env.DISASM_PATH ?? path.join(ROOT_DIR, "disasm.txt");
const LOG_PATH = process.env.LOG_PATH ?? path.join(ROOT_DIR, "emu.log");

app.use(express.static(PUBLIC_DIR));

console.log("DISASM_PATH=", DISASM_PATH);
console.log("LOG_PATH=", LOG_PATH);
console.log("PUBLIC_DIR=", PUBLIC_DIR);

// --- helpers -------------------------------------------------------------

function escapeHtml(s: string): string {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

type DisasmLine = {
  raw: string;
  addr?: bigint; // 命令行なら addr を持つ
};

function parseDisasm(text: string): DisasmLine[] {
  // 例: "    800090b0:\t12000073\t..."
  const insnRe = /^\s*([0-9A-Fa-f]+):\s+/;

  return text.split(/\r?\n/).map((raw) => {
    const m = raw.match(insnRe);
    if (!m) return { raw };
    return { raw, addr: BigInt("0x" + m[1]) };
  });
}

function renderDisasmHtml(
  disasmLines: DisasmLine[],
  start?: bigint,
  end?: bigint
): string {
  // start/end がある場合、その範囲の命令行を hl class で強調
  // 範囲は [start, end) を採用
  let firstHlId: string | null = null;

  const out: string[] = [];
  out.push(`<pre class="disasm">`);

  disasmLines.forEach((ln, idx) => {
    const safe = escapeHtml(ln.raw);
    if (start !== undefined && end !== undefined && ln.addr !== undefined) {
      const inRange = ln.addr >= start && ln.addr <= end;
      if (inRange) {
        const id = `hl-${idx}`;
        if (!firstHlId) firstHlId = id;
        out.push(`<span id="${id}" class="hl">${safe}</span>`);
        return;
      }
    }
    out.push(safe);
  });

  out.push(`</pre>`);
  // 先頭ハイライト行をクライアントへ伝える（data属性）
  const first = firstHlId ? ` data-first-hl="${firstHlId}"` : "";
  return `<div class="disasmWrap"${first}>${out.join("\n")}</div>`;
}

// --- data load (simple: read files on each request; optimize if needed) ---

function readFileOrEmpty(p: string): string {
  try {
    return fs.readFileSync(p, "utf-8");
  } catch {
    return "";
  }
}

// --- routes --------------------------------------------------------------
// CSPを明示（htmxをunpkgから読む前提）
app.use((req, res, next) => {
  res.setHeader(
    "Content-Security-Policy",
    [
      "default-src 'self'",
      "script-src 'self' https://unpkg.com",  // htmx CDN
      "style-src 'self' 'unsafe-inline'",     // index.html内のstyle用
      "img-src 'self' data:",
      "font-src 'self' data:",
      "connect-src 'self'",
      "base-uri 'self'",
      "frame-ancestors 'self'",
    ].join("; ")
  );
  next();
});

app.use(express.static(path.join(__dirname, "public")));

app.get("/api/logs", (req, res) => {
  const text = readFileOrEmpty(LOG_PATH);

  // 「Block execution:」を含む行を拾う + start/end は取れれば取る
  const re = /Block execution:\s*(0x[0-9A-Fa-f]+)?\s*(?:to\s*(0x[0-9A-Fa-f]+))?/;

  const items = text
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .map((line) => {
      if (!line.includes("Block execution:")) return null;
      const m = line.match(re);
      return { line, start: m?.[1] ?? "", end: m?.[2] ?? "" };
    })
    .filter((x): x is { line: string; start: string; end: string } => x !== null);

  const html = items
    .map((it) => {
      const url =
        it.start && it.end
          ? `/api/disasm?start=${encodeURIComponent(it.start)}&end=${encodeURIComponent(it.end)}`
          : `/api/disasm`;

      return `
<button
  class="logLine"
  type="button"
  hx-get="${url}"
  hx-target="#disasmPanel"
  hx-swap="innerHTML"
>
  ${escapeHtml(it.line)}
</button>`.trim();
    })
    .join("\n");

  res.type("text/html").send(html || `<div class="muted">No matching log lines.</div>`);
});

app.get("/api/disasm", (req, res) => {
  const disasmText = readFileOrEmpty(DISASM_PATH);
  const lines = parseDisasm(disasmText);

  const startQ = typeof req.query.start === "string" ? req.query.start : undefined;
  const endQ = typeof req.query.end === "string" ? req.query.end : undefined;

  let start: bigint | undefined;
  let end: bigint | undefined;

  try {
    if (startQ && endQ) {
      start = BigInt(startQ);
      end = BigInt(endQ);
    }
  } catch {
    // ignore; render without highlight
  }

  const html = renderDisasmHtml(lines, start, end);
  res.type("text/html").send(html);
});

app.listen(PORT, () => {
  console.log(`open http://localhost:${PORT}`);
  console.log(`DISASM_PATH=${DISASM_PATH}`);
  console.log(`LOG_PATH=${LOG_PATH}`);
});
