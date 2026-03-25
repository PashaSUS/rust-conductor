/** Try to parse a string value into a typed value (number, boolean, object/array, or string) */
export function parseFieldValue(raw: string): unknown {
  const trimmed = raw.trim();
  if (trimmed === "") return "";
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null") return null;
  const num = Number(trimmed);
  if (!isNaN(num) && trimmed !== "") return num;
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try {
      return JSON.parse(trimmed);
    } catch {
      // fall through to string
    }
  }
  return raw;
}

/** Build an input object from field values, parsing each to its inferred type */
export function buildInputFromFields(
  fields: Record<string, string>,
): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(fields)) {
    result[key] = parseFieldValue(val);
  }
  return result;
}
