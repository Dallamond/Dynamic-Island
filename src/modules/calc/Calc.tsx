// Calculadora típica. Funciona con clics sin robar el foco; con el teclado tras hacer clic en ella.
import { Delete } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { evaluate, formatNumber } from "./evaluate";

const KEYS = [
  ["7", "8", "9", "÷", "C"],
  ["4", "5", "6", "×", "⌫"],
  ["1", "2", "3", "−", "%"],
  ["0", ",", "±", "+", "="],
];
const OPS = "+−×÷";

// Se conserva entre aperturas del panel (pero no entre reinicios).
let saved = { expr: "", last: "" };

export function CalcPanel() {
  const [expr, setExpr] = useState(saved.expr);
  const [last, setLast] = useState(saved.last); // expresión del último "="
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    saved = { expr, last };
  }, [expr, last]);

  const press = (k: string) => {
    if (k === "C") {
      setExpr("");
      setLast("");
    } else if (k === "⌫") setExpr((e) => e.slice(0, -1));
    else if (k === "=") {
      const r = evaluate(expr);
      if (r !== null) {
        setLast(expr);
        setExpr(String(r).replace(".", ","));
      }
    } else if (k === "±") {
      // Cambia el signo del último número.
      setExpr((e) => {
        const m = e.match(/(-?)([\d,]+)$/);
        if (!m) return e;
        const start = e.length - m[0].length;
        const before = e.slice(0, start);
        const neg = m[1] === "-" && (start === 0 || OPS.includes(before.slice(-1)));
        return neg ? before + m[2] : before + "-" + m[2];
      });
    } else if (OPS.includes(k)) {
      setExpr((e) => {
        if (!e) return k === "−" ? "-" : e;
        // Sustituir operador final en vez de encadenarlo (salvo "×−" para negativos).
        if (OPS.includes(e.slice(-1)) && !(k === "−" && "×÷".includes(e.slice(-1)))) return e.slice(0, -1) + k;
        return e + k;
      });
    } else if (k === ",") {
      setExpr((e) => (/[\d]*,[\d]*$/.test(e.split(/[+−×÷-]/).pop() ?? "") ? e : e + (/\d$/.test(e) ? "," : "0,")));
    } else setExpr((e) => e + k);
  };

  const onKey = (e: React.KeyboardEvent) => {
    const map: Record<string, string> = { "*": "×", "/": "÷", "-": "−", "+": "+", Enter: "=", "=": "=", Backspace: "⌫", Escape: "C", Delete: "C", ".": ",", ",": ",", "%": "%" };
    const k = /^\d$/.test(e.key) ? e.key : map[e.key];
    if (k) {
      e.preventDefault();
      press(k);
    }
  };

  const preview = OPS.includes(expr.slice(-1)) ? null : evaluate(expr);
  const showPreview = preview !== null && expr !== String(preview).replace(".", ",") && /[+−×÷%]/.test(expr.slice(1));

  return (
    <div className="calc" ref={box} tabIndex={0} data-keyboard onKeyDown={onKey}>
      <div className="calc-display">
        <div className="calc-last muted">{showPreview ? `= ${formatNumber(preview!)}` : last ? `${last} =` : " "}</div>
        <div className="calc-expr">{expr || "0"}</div>
      </div>
      <div className="calc-grid">
        {KEYS.flat().map((k) => (
          <button
            key={k}
            className={`calc-key${OPS.includes(k) ? " op" : ""}${k === "=" ? " eq" : ""}${"C⌫%±".includes(k) ? " fn" : ""}`}
            onClick={() => {
              press(k);
              box.current?.focus();
            }}
          >
            {k === "⌫" ? <Delete size={16} /> : k}
          </button>
        ))}
      </div>
    </div>
  );
}
