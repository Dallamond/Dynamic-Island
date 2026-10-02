// Evaluador de expresiones de calculadora: + − × ÷, %, decimales y signo. Sin eval().
// Precedencia habitual (× ÷ antes que + −). "50+10%" = 55, como en las calculadoras de bolsillo.

type Tok = { t: "num"; v: number } | { t: "op"; v: string };

function tokenize(src: string): Tok[] | null {
  const out: Tok[] = [];
  const s = src.replace(/×/g, "*").replace(/÷/g, "/").replace(/−/g, "-").replace(/,/g, ".").replace(/\s+/g, "");
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    const prev = out[out.length - 1];
    const unary = c === "-" && (!prev || (prev.t === "op" && prev.v !== "%"));
    if (/[\d.]/.test(c) || unary) {
      let j = i + 1;
      while (j < s.length && /[\d.]/.test(s[j])) j++;
      const v = Number(s.slice(i, j));
      if (Number.isNaN(v)) return null;
      out.push({ t: "num", v });
      i = j;
    } else if ("+-*/%".includes(c)) {
      out.push({ t: "op", v: c });
      i++;
    } else return null;
  }
  return out;
}

/** Devuelve el resultado o `null` si la expresión está incompleta o es inválida. */
export function evaluate(src: string): number | null {
  const toks = tokenize(src);
  if (!toks || toks.length === 0) return null;
  // Términos sumados: cada término es un producto/división.
  let total = 0;
  let term: number | null = null;
  let sign = 1;
  let pendingMul: string | null = null;
  let lastAdd: number | null = null; // valor acumulado antes del término actual (para %)
  for (let k = 0; k < toks.length; k++) {
    const tk = toks[k];
    if (tk.t === "num") {
      let v = tk.v;
      if (toks[k + 1]?.t === "op" && toks[k + 1].v === "%") {
        // x% → sobre una suma/resta es % del acumulado; en un producto es x/100.
        v = pendingMul === null && term === null && lastAdd !== null ? (lastAdd * v) / 100 : v / 100;
        k++;
      }
      if (term === null) term = v;
      else if (pendingMul === "*") term *= v;
      else if (pendingMul === "/") {
        if (v === 0) return null;
        term /= v;
      } else return null;
      pendingMul = null;
    } else {
      if (term === null) return null;
      if (tk.v === "+" || tk.v === "-") {
        total += sign * term;
        lastAdd = total;
        sign = tk.v === "+" ? 1 : -1;
        term = null;
      } else if (tk.v === "*" || tk.v === "/") {
        pendingMul = tk.v;
      } else return null;
    }
  }
  if (term === null || pendingMul !== null) return null;
  const r = total + sign * term;
  return Number.isFinite(r) ? Math.round(r * 1e10) / 1e10 : null;
}

export function formatNumber(n: number): string {
  const abs = Math.abs(n);
  if (abs !== 0 && (abs >= 1e12 || abs < 1e-6)) return n.toExponential(6).replace(/\.?0+e/, "e");
  return new Intl.NumberFormat("es-ES", { maximumFractionDigits: 10 }).format(n);
}
