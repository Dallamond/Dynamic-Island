// Notas rápidas: se guardan en Rust (notes.json) en cada cambio.
import { Check, Copy, X } from "lucide-react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Note {
  id: string;
  text: string;
  done: boolean;
  createdMs: number;
}

export function NotesPanel() {
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [draft, setDraft] = useState("");
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    invoke<Note[]>("notes_get").then(setNotes).catch(() => setNotes([]));
  }, []);

  const save = (next: Note[]) => {
    setNotes(next);
    invoke("notes_save", { notes: next });
  };

  if (!notes) return null;

  const add = () => {
    const text = draft.trim();
    if (!text) return;
    save([{ id: crypto.randomUUID(), text, done: false, createdMs: Date.now() }, ...notes]);
    setDraft("");
  };

  return (
    <div className="notes">
      <input
        className="notes-input"
        placeholder="Escribe una nota y pulsa Enter…"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && add()}
      />
      <div className="notes-list">
        {notes.length === 0 && <p className="muted notes-empty">Sin notas.</p>}
        {notes.map((n) => (
          <div key={n.id} className={`note${n.done ? " done" : ""}`}>
            <button className="note-check" title="Hecha" onClick={() => save(notes.map((x) => (x.id === n.id ? { ...x, done: !x.done } : x)))}>
              {n.done && <Check size={11} strokeWidth={3} />}
            </button>
            <span className="note-text">{n.text}</span>
            <button
              className="note-act"
              title="Copiar"
              onClick={() => {
                navigator.clipboard.writeText(n.text);
                setCopied(n.id);
                window.setTimeout(() => setCopied(null), 1200);
              }}
            >
              {copied === n.id ? <Check size={13} /> : <Copy size={13} />}
            </button>
            <button className="note-act" title="Borrar" onClick={() => save(notes.filter((x) => x.id !== n.id))}>
              <X size={14} />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
