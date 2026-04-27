import { useEffect, useMemo, useRef, useState } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Check, AlertCircle, Wand2 } from "lucide-react";

/**
 * Tokenize a JSON-ish string into highlightable spans.
 *
 * This is intentionally permissive — we tokenize *while the user is typing*,
 * which means the input is frequently invalid. Tokens that can't be matched
 * fall through as plain text. Comments (// and /* *\/) are tolerated even
 * though strict JSON disallows them; users sometimes paste annotated samples.
 */
type Tok =
  | { kind: "key"; v: string }
  | { kind: "string"; v: string }
  | { kind: "number"; v: string }
  | { kind: "bool"; v: string }
  | { kind: "null"; v: string }
  | { kind: "punct"; v: string }
  | { kind: "comment"; v: string }
  | { kind: "ws"; v: string }
  | { kind: "text"; v: string };

function tokenize(src: string): Tok[] {
  const out: Tok[] = [];
  let i = 0;
  const n = src.length;
  // Track whether the most-recent non-ws token before the current string is `{` or `,`
  // (used to colour object keys differently from value strings).
  let lastSignificant: string | null = null;

  while (i < n) {
    const c = src[i];

    // whitespace
    if (c === " " || c === "\t" || c === "\n" || c === "\r") {
      let j = i;
      while (j < n && (src[j] === " " || src[j] === "\t" || src[j] === "\n" || src[j] === "\r")) j++;
      out.push({ kind: "ws", v: src.slice(i, j) });
      i = j;
      continue;
    }

    // line comment
    if (c === "/" && src[i + 1] === "/") {
      let j = i;
      while (j < n && src[j] !== "\n") j++;
      out.push({ kind: "comment", v: src.slice(i, j) });
      i = j;
      continue;
    }
    // block comment
    if (c === "/" && src[i + 1] === "*") {
      let j = i + 2;
      while (j < n && !(src[j] === "*" && src[j + 1] === "/")) j++;
      j = Math.min(n, j + 2);
      out.push({ kind: "comment", v: src.slice(i, j) });
      i = j;
      continue;
    }

    // string (double-quoted, with escape handling)
    if (c === '"') {
      let j = i + 1;
      while (j < n) {
        if (src[j] === "\\" && j + 1 < n) { j += 2; continue; }
        if (src[j] === '"') { j++; break; }
        if (src[j] === "\n") break; // unterminated
        j++;
      }
      const v = src.slice(i, j);
      // Look ahead past whitespace for `:` to decide if this is a key
      let k = j;
      while (k < n && (src[k] === " " || src[k] === "\t")) k++;
      const isKey =
        src[k] === ":" && (lastSignificant === "{" || lastSignificant === "," || lastSignificant === null);
      out.push({ kind: isKey ? "key" : "string", v });
      lastSignificant = '"';
      i = j;
      continue;
    }

    // number
    if (c === "-" || (c >= "0" && c <= "9")) {
      let j = i;
      if (src[j] === "-") j++;
      while (j < n && /[0-9.eE+-]/.test(src[j])) j++;
      out.push({ kind: "number", v: src.slice(i, j) });
      lastSignificant = "0";
      i = j;
      continue;
    }

    // keywords
    if (src.startsWith("true", i)) { out.push({ kind: "bool", v: "true" }); lastSignificant = "t"; i += 4; continue; }
    if (src.startsWith("false", i)) { out.push({ kind: "bool", v: "false" }); lastSignificant = "f"; i += 5; continue; }
    if (src.startsWith("null", i)) { out.push({ kind: "null", v: "null" }); lastSignificant = "n"; i += 4; continue; }

    // punctuation
    if ("{}[]:,".includes(c)) {
      out.push({ kind: "punct", v: c });
      lastSignificant = c;
      i++;
      continue;
    }

    // fallback
    out.push({ kind: "text", v: c });
    i++;
  }

  return out;
}

const TOKEN_CLASS: Record<Tok["kind"], string> = {
  key: "text-[var(--json-key,oklch(0.55_0.18_280))] dark:text-[var(--json-key-dark,oklch(0.78_0.16_300))]",
  string: "text-[var(--json-string,oklch(0.55_0.16_140))] dark:text-[var(--json-string-dark,oklch(0.78_0.14_140))]",
  number: "text-[var(--json-number,oklch(0.55_0.20_30))] dark:text-[var(--json-number-dark,oklch(0.78_0.18_30))]",
  bool: "text-[var(--json-bool,oklch(0.50_0.22_240))] dark:text-[var(--json-bool-dark,oklch(0.75_0.18_240))] font-semibold",
  null: "text-muted-foreground italic",
  punct: "text-foreground/60",
  comment: "text-muted-foreground italic",
  ws: "",
  text: "text-destructive",
};

interface ValidationResult {
  ok: boolean;
  error?: string;
  line?: number;
  column?: number;
}

function validate(src: string): ValidationResult {
  if (!src.trim()) return { ok: true };
  try {
    JSON.parse(src);
    return { ok: true };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    // Try to extract `position N` from V8/SpiderMonkey messages and turn it into line:col
    const posMatch = /position (\d+)/i.exec(msg);
    if (posMatch) {
      const pos = Number(posMatch[1]);
      let line = 1, col = 1;
      for (let i = 0; i < pos && i < src.length; i++) {
        if (src[i] === "\n") { line++; col = 1; } else col++;
      }
      return { ok: false, error: msg, line, column: col };
    }
    const lineMatch = /line (\d+) column (\d+)/i.exec(msg);
    if (lineMatch) {
      return { ok: false, error: msg, line: Number(lineMatch[1]), column: Number(lineMatch[2]) };
    }
    return { ok: false, error: msg };
  }
}

export interface JsonEditorProps {
  value: string;
  onChange: (v: string) => void;
  /** rows-equivalent height. Defaults to 10. */
  rows?: number;
  placeholder?: string;
  disabled?: boolean;
  /** Show the [Format] button. Defaults to true. */
  showFormat?: boolean;
  /** Show the validation badge. Defaults to true. */
  showValidation?: boolean;
  className?: string;
  /** Optional aria label for the textarea. */
  ariaLabel?: string;
}

/**
 * JSON editor with inline syntax highlighting, line numbers and live validation.
 *
 * Implementation: a transparent <textarea> sits on top of a syntax-highlighted
 * <pre>; both share the same monospace font + line-height so the caret aligns
 * with the visible glyphs. The textarea owns input and selection; the <pre>
 * is purely visual.
 */
export function JsonEditor({
  value,
  onChange,
  rows = 10,
  placeholder,
  disabled,
  showFormat = true,
  showValidation = true,
  className,
  ariaLabel = "JSON editor",
}: JsonEditorProps) {
  const taRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);
  const [scroll, setScroll] = useState({ top: 0, left: 0 });

  const tokens = useMemo(() => tokenize(value), [value]);
  const validation = useMemo(() => validate(value), [value]);
  const lineCount = useMemo(() => Math.max(1, value.split("\n").length), [value]);

  // Keep the highlighted layer scrolled in sync with the textarea
  useEffect(() => {
    const ta = taRef.current;
    if (!ta) return;
    const onScroll = () => setScroll({ top: ta.scrollTop, left: ta.scrollLeft });
    ta.addEventListener("scroll", onScroll, { passive: true });
    return () => ta.removeEventListener("scroll", onScroll);
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const ta = e.currentTarget;
    // Tab → 2 spaces (or dedent on shift+tab at line start)
    if (e.key === "Tab") {
      e.preventDefault();
      const start = ta.selectionStart;
      const end = ta.selectionEnd;
      if (e.shiftKey) {
        // Dedent: remove up to 2 leading spaces from each selected line
        const lineStart = value.lastIndexOf("\n", start - 1) + 1;
        const before = value.slice(0, lineStart);
        const block = value.slice(lineStart, end);
        const dedented = block.replace(/^ {1,2}/gm, "");
        const newVal = before + dedented + value.slice(end);
        onChange(newVal);
        const removed = block.length - dedented.length;
        requestAnimationFrame(() => {
          ta.selectionStart = Math.max(lineStart, start - Math.min(2, removed));
          ta.selectionEnd = end - removed;
        });
      } else {
        const insert = "  ";
        const newVal = value.slice(0, start) + insert + value.slice(end);
        onChange(newVal);
        requestAnimationFrame(() => {
          ta.selectionStart = ta.selectionEnd = start + insert.length;
        });
      }
      return;
    }
    // Enter → continue indent of current line
    if (e.key === "Enter") {
      const start = ta.selectionStart;
      const lineStart = value.lastIndexOf("\n", start - 1) + 1;
      const linePrefix = value.slice(lineStart, start).match(/^[ \t]*/)?.[0] ?? "";
      // If previous non-space char is `{` or `[`, add an extra level
      const prev = value.slice(0, start).replace(/\s+$/, "").slice(-1);
      const extra = prev === "{" || prev === "[" ? "  " : "";
      if (linePrefix.length > 0 || extra) {
        e.preventDefault();
        const insert = "\n" + linePrefix + extra;
        const newVal = value.slice(0, start) + insert + value.slice(ta.selectionEnd);
        onChange(newVal);
        requestAnimationFrame(() => {
          ta.selectionStart = ta.selectionEnd = start + insert.length;
        });
      }
    }
  };

  const handleFormat = () => {
    try {
      const parsed = JSON.parse(value || "{}");
      onChange(JSON.stringify(parsed, null, 2));
    } catch {
      // ignore — validation badge already shows the error
    }
  };

  // Build line-number gutter
  const lineNumbers = useMemo(() => {
    const arr: string[] = [];
    for (let i = 1; i <= lineCount; i++) arr.push(String(i));
    return arr;
  }, [lineCount]);

  // Approx height: rows * 1.5em line-height + 1em padding
  const minHeightPx = rows * 21 + 16;

  return (
    <div className={cn("space-y-1", className)}>
      <div
        className={cn(
          "relative rounded-md border bg-muted/30 overflow-hidden font-mono",
          "focus-within:ring-2 focus-within:ring-ring focus-within:border-ring",
          disabled && "opacity-50 pointer-events-none",
          !validation.ok && "border-destructive/60",
        )}
      >
        <div className="flex" style={{ minHeight: minHeightPx }}>
          {/* Gutter */}
          <pre
            aria-hidden
            className="select-none text-right text-[11px] leading-5.25 py-2 px-2 text-muted-foreground/60 bg-muted/40 border-r"
            style={{
              transform: `translateY(${-scroll.top}px)`,
              minWidth: `${String(lineCount).length + 1}ch`,
            }}
          >
            {lineNumbers.join("\n")}
          </pre>

          {/* Editor stack */}
          <div className="relative flex-1 overflow-hidden">
            <pre
              ref={preRef}
              aria-hidden
              className="absolute inset-0 m-0 px-3 py-2 text-xs leading-5.25 whitespace-pre-wrap wrap-break-word pointer-events-none"
              style={{
                transform: `translate(${-scroll.left}px, ${-scroll.top}px)`,
              }}
            >
              {tokens.map((tok, idx) => (
                <span key={idx} className={TOKEN_CLASS[tok.kind]}>
                  {tok.v}
                </span>
              ))}
              {/* Trailing newline so caret on last empty line is visible */}
              {value.endsWith("\n") && "\n"}
            </pre>
            <textarea
              ref={taRef}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              onKeyDown={handleKeyDown}
              spellCheck={false}
              aria-label={ariaLabel}
              placeholder={placeholder}
              disabled={disabled}
              className={cn(
                "relative block w-full px-3 py-2 text-xs leading-5.25",
                "bg-transparent text-transparent caret-foreground resize-y",
                "outline-none border-0 whitespace-pre-wrap wrap-break-word",
                "selection:bg-primary/30 selection:text-foreground",
              )}
              style={{
                minHeight: minHeightPx,
                // Make the textarea's value invisible (we render via <pre>) but
                // keep the placeholder readable.
                WebkitTextFillColor: "transparent",
              }}
            />
          </div>
        </div>
      </div>

      {(showValidation || showFormat) && (
        <div className="flex items-center justify-between text-[10px]">
          {showValidation ? (
            validation.ok ? (
              <span className="inline-flex items-center gap-1 text-green-600 dark:text-green-500">
                <Check className="h-3 w-3" /> Valid JSON
              </span>
            ) : (
              <span className="inline-flex items-center gap-1 text-destructive truncate" title={validation.error}>
                <AlertCircle className="h-3 w-3 shrink-0" />
                {validation.line ? `Line ${validation.line}${validation.column ? `:${validation.column}` : ""} — ` : ""}
                <span className="truncate">{validation.error}</span>
              </span>
            )
          ) : <span />}
          {showFormat && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-[10px] gap-1"
              onClick={handleFormat}
              disabled={disabled || !validation.ok}
              title="Format JSON (Pretty-print)"
            >
              <Wand2 className="h-3 w-3" /> Format
            </Button>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * Build a JSON template object from a list of parameter keys, optionally
 * preserving values from a previously-edited object.
 *
 * Used by dialogs to pre-fill the JSON editor with the workflow/task input
 * schema so the user only has to fill in values rather than retype keys.
 */
export function buildJsonTemplate(
  keys: readonly (string | unknown)[],
  existing?: Record<string, unknown> | null,
): string {
  const obj: Record<string, unknown> = {};
  for (const k of keys) {
    if (typeof k !== "string") continue;
    obj[k] = existing && k in existing ? existing[k] : "";
  }
  return JSON.stringify(obj, null, 2);
}
