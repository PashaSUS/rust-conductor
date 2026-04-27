// ── Random data generators for stress testing ──

function randInt(min: number, max: number) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}
function randFloat(min: number, max: number) {
  return +(Math.random() * (max - min) + min).toFixed(2);
}
export function randPick<T>(arr: T[]): T {
  return arr[randInt(0, arr.length - 1)];
}
function randBool() {
  return Math.random() > 0.5;
}
function randString(len: number) {
  const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
  return Array.from({ length: len }, () => randPick(chars.split(""))).join("");
}
export function randWord() {
  const words = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
    "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa",
    "quebec", "romeo", "sierra", "tango", "uniform", "victor", "whiskey",
    "xray", "yankee", "zulu", "phoenix", "thunder", "storm", "blaze",
    "rocket", "nebula", "comet", "aurora", "glacier", "canyon", "summit",
  ];
  return randPick(words);
}
function randName() {
  const first = ["Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Hank", "Ivy", "Jack"];
  const last = ["Smith", "Jones", "Brown", "Wilson", "Taylor", "Clark", "Moore", "White", "King", "Hall"];
  return `${randPick(first)} ${randPick(last)}`;
}
function randEmail() {
  return `${randWord()}.${randWord()}@${randWord()}.com`;
}
function randUUID() {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
  });
}
function randIP() {
  return `${randInt(10, 192)}.${randInt(0, 255)}.${randInt(0, 255)}.${randInt(1, 254)}`;
}
export function randUrl() {
  const protos = ["https"];
  const tlds = ["com", "io", "dev", "org", "net"];
  return `${randPick(protos)}://${randWord()}.${randPick(tlds)}/api/${randWord()}`;
}

export type ValueType = "string" | "number" | "boolean" | "email" | "uuid" | "ip" | "url" | "name" | "json_object" | "json_array" | "regex";

export const VALUE_TYPES: { value: ValueType; label: string }[] = [
  { value: "string", label: "Random String" },
  { value: "number", label: "Random Number" },
  { value: "boolean", label: "Random Boolean" },
  { value: "email", label: "Random Email" },
  { value: "uuid", label: "Random UUID" },
  { value: "ip", label: "Random IP Address" },
  { value: "url", label: "Random URL" },
  { value: "name", label: "Random Name" },
  { value: "json_object", label: "Random JSON Object" },
  { value: "json_array", label: "Random JSON Array" },
  { value: "regex", label: "Regex Pattern" },
];

export function generateValue(type: ValueType): unknown {
  switch (type) {
    case "string": return `${randWord()}_${randString(6)}`;
    case "number": return randFloat(0, 10000);
    case "boolean": return randBool();
    case "email": return randEmail();
    case "uuid": return randUUID();
    case "ip": return randIP();
    case "url": return randUrl();
    case "name": return randName();
    case "json_object": return {
      id: randUUID(),
      name: randName(),
      value: randFloat(0, 100),
      active: randBool(),
      tags: [randWord(), randWord()],
    };
    case "json_array": return Array.from({ length: randInt(2, 5) }, () => ({
      key: randWord(),
      value: randFloat(0, 1000),
    }));
    default: return randString(10);
  }
}
