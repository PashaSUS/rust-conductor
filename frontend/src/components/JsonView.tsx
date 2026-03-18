import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { CopyButton } from "@/components/CopyButton";

interface JsonViewProps {
  data: unknown;
  className?: string;
  maxHeight?: string;
  copyable?: boolean;
}

interface Token {
  type: "key" | "string" | "number" | "boolean" | "null" | "brace" | "bracket" | "colon" | "comma";
  value: string;
}

function tokenize(json: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;
  let expectKey = false;

  while (i < json.length) {
    const ch = json[i];

    if (ch === " " || ch === "\n" || ch === "\r" || ch === "\t") {
      // preserve whitespace as-is
      let ws = "";
      while (i < json.length && (json[i] === " " || json[i] === "\n" || json[i] === "\r" || json[i] === "\t")) {
        ws += json[i++];
      }
      tokens.push({ type: "string", value: ws }); // whitespace uses no special class
      continue;
    }

    if (ch === "{" || ch === "}") {
      tokens.push({ type: "brace", value: ch });
      expectKey = ch === "{";
      i++;
      continue;
    }

    if (ch === "[" || ch === "]") {
      tokens.push({ type: "bracket", value: ch });
      i++;
      continue;
    }

    if (ch === ":") {
      tokens.push({ type: "colon", value: ": " });
      expectKey = false;
      i++;
      // skip space after colon
      if (i < json.length && json[i] === " ") i++;
      continue;
    }

    if (ch === ",") {
      tokens.push({ type: "comma", value: "," });
      expectKey = true;
      i++;
      continue;
    }

    if (ch === '"') {
      let str = '"';
      i++;
      while (i < json.length && json[i] !== '"') {
        if (json[i] === "\\") { str += json[i++]; }
        str += json[i++];
      }
      str += '"';
      i++;
      tokens.push({ type: expectKey ? "key" : "string", value: str });
      continue;
    }

    // Numbers, booleans, null
    let word = "";
    while (i < json.length && !/[\s,\]\}]/.test(json[i])) {
      word += json[i++];
    }
    if (word === "true" || word === "false") {
      tokens.push({ type: "boolean", value: word });
    } else if (word === "null") {
      tokens.push({ type: "null", value: word });
    } else {
      tokens.push({ type: "number", value: word });
    }
  }

  return tokens;
}

const TOKEN_CLASSES: Record<Token["type"], string> = {
  key: "text-blue-600 dark:text-blue-400",
  string: "text-emerald-600 dark:text-emerald-400",
  number: "text-amber-600 dark:text-amber-400",
  boolean: "text-purple-600 dark:text-purple-400",
  null: "text-red-500 dark:text-red-400",
  brace: "text-foreground/60",
  bracket: "text-foreground/60",
  colon: "text-foreground/40",
  comma: "text-foreground/40",
};

export function JsonView({ data, className, maxHeight = "24rem", copyable = true }: JsonViewProps) {
  const jsonStr = useMemo(() => {
    try {
      return JSON.stringify(data, null, 2);
    } catch {
      return String(data);
    }
  }, [data]);

  const tokens = useMemo(() => tokenize(jsonStr), [jsonStr]);

  if (data === undefined || data === null) {
    return <span className="text-xs text-muted-foreground italic">null</span>;
  }

  return (
    <div className={cn("relative group rounded-lg bg-muted", className)}>
      {copyable && (
        <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
          <CopyButton value={jsonStr} />
        </div>
      )}
      <pre
        className="text-xs p-3 overflow-auto font-mono leading-relaxed"
        style={{ maxHeight }}
      >
        {tokens.map((token, i) => {
          const cls = TOKEN_CLASSES[token.type];
          // Whitespace tokens don't need styling
          if (!cls || token.value.trim() === "") {
            return <span key={i}>{token.value}</span>;
          }
          return (
            <span key={i} className={cls}>
              {token.value}
            </span>
          );
        })}
      </pre>
    </div>
  );
}
