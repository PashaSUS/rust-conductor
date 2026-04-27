import { randPick } from "./random-generators";

// ── Simple regex-to-string generator ──
// Supports: literal chars, [a-z] [A-Z] [0-9] char classes, \d \w \s shortcuts,
// {n} {n,m} quantifiers, + * ?, (group), | alternation, . wildcard
export function generateFromRegex(pattern: string): string {
  let pos = 0;
  const src = pattern;

  function peek() { return src[pos]; }
  function next() { return src[pos++]; }
  function hasMore() { return pos < src.length; }

  function charRange(a: string, b: string): string[] {
    const result: string[] = [];
    for (let c = a.charCodeAt(0); c <= b.charCodeAt(0); c++) result.push(String.fromCharCode(c));
    return result;
  }

  const DIGITS = charRange("0", "9");
  const LOWER = charRange("a", "z");
  const UPPER = charRange("A", "Z");
  const WORD = [...LOWER, ...UPPER, ...DIGITS, "_"];
  const PRINTABLE = [...WORD, " ", "!", "@", "#", "$", "%", "&", "*", "-", "+", "=", ".", ",", ";", ":"];

  function randBool() { return Math.random() > 0.5; }
  function randInt(min: number, max: number) { return Math.floor(Math.random() * (max - min + 1)) + min; }

  function parseCharClass(): string[] {
    const chars: string[] = [];
    const negated = peek() === "^";
    if (negated) next();
    while (hasMore() && peek() !== "]") {
      const c = next()!;
      if (c === "\\" && hasMore()) {
        const esc = next()!;
        if (esc === "d") chars.push(...DIGITS);
        else if (esc === "w") chars.push(...WORD);
        else if (esc === "s") chars.push(" ", "\t");
        else chars.push(esc);
      } else if (peek() === "-" && pos + 1 < src.length && src[pos + 1] !== "]") {
        next();
        const end = next()!;
        chars.push(...charRange(c, end));
      } else {
        chars.push(c);
      }
    }
    if (peek() === "]") next();
    if (negated) {
      const set = new Set(chars);
      return PRINTABLE.filter((c) => !set.has(c));
    }
    return chars.length > 0 ? chars : ["a"];
  }

  function parseAtom(): () => string {
    if (!hasMore()) return () => "";
    const c = peek()!;
    if (c === "(") { next(); const inner = parseAlternation(); if (peek() === ")") next(); return inner; }
    if (c === "[") { next(); const chars = parseCharClass(); return () => randPick(chars); }
    if (c === "\\") {
      next();
      const esc = next()!;
      if (esc === "d") return () => randPick(DIGITS);
      if (esc === "w") return () => randPick(WORD);
      if (esc === "s") return () => randPick([" ", "\t"]);
      return () => esc;
    }
    if (c === ".") { next(); return () => randPick(PRINTABLE); }
    next();
    return () => c;
  }

  function parseQuantifier(atom: () => string): () => string {
    if (!hasMore()) return atom;
    const c = peek();
    if (c === "*") { next(); const n = randInt(0, 5); return () => Array.from({ length: n }, () => atom()).join(""); }
    if (c === "+") { next(); const n = randInt(1, 5); return () => Array.from({ length: n }, () => atom()).join(""); }
    if (c === "?") { next(); return () => (randBool() ? atom() : ""); }
    if (c === "{") {
      const saved = pos;
      next();
      let numStr = "";
      while (hasMore() && /\d/.test(peek()!)) numStr += next();
      if (peek() === "}") {
        next();
        const n = parseInt(numStr, 10) || 1;
        return () => Array.from({ length: n }, () => atom()).join("");
      }
      if (peek() === ",") {
        next();
        let maxStr = "";
        while (hasMore() && /\d/.test(peek()!)) maxStr += next();
        if (peek() === "}") {
          next();
          const min = parseInt(numStr, 10) || 0;
          const max = maxStr ? parseInt(maxStr, 10) : min + 5;
          return () => { const n = randInt(min, max); return Array.from({ length: n }, () => atom()).join(""); };
        }
      }
      pos = saved;
    }
    return atom;
  }

  function parseSequence(): () => string {
    const parts: (() => string)[] = [];
    while (hasMore() && peek() !== ")" && peek() !== "|") {
      const atom = parseAtom();
      parts.push(parseQuantifier(atom));
    }
    return () => parts.map((p) => p()).join("");
  }

  function parseAlternation(): () => string {
    const alternatives: (() => string)[] = [parseSequence()];
    while (hasMore() && peek() === "|") { next(); alternatives.push(parseSequence()); }
    return () => randPick(alternatives)();
  }

  const gen = parseAlternation();
  return gen();
}
