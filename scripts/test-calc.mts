// Tests del evaluador de la calculadora: node scripts/test-calc.mts
import { evaluate } from "../src/modules/calc/evaluate.ts";
const cases: [string, number | null][] = [["2+3×4", 14], ["10÷4", 2.5], ["50+10%", 55], ["200×10%", 20], ["-3+5", 2], ["2×-3", -6], ["1÷0", null], ["2+", null], ["0,1+0,2", 0.3], ["100−20%", 80], ["7", 7]];
let fail = 0;
for (const [e, want] of cases) { const got = evaluate(e); if (got !== want) { fail++; console.log("FALLA", e, got, want); } }
console.log(fail ? `${fail} fallos` : `ok ${cases.length} casos`);
