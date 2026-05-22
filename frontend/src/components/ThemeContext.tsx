import { createContext, useContext, type ReactNode } from "react";
import { defaultText } from "./themes";
export type { UITheme, ThemeText } from "./themes";
import type { UITheme, ThemeText } from "./themes";

/* ------------------------------------------------------------------ */
/*  Context — themes have been removed. Always returns the default     */
/*  string table. This file is kept as a shim so existing imports of   */
/*  `useThemeText` / `useTheme` continue to work without touching       */
/*  every call site.                                                   */
/* ------------------------------------------------------------------ */

interface ThemeContextValue {
  uiTheme: UITheme;
  setUITheme: (theme: UITheme) => void;
  t: ThemeText;
}

const ThemeContext = createContext<ThemeContextValue>({
  uiTheme: "default",
  setUITheme: () => {},
  t: defaultText,
});

export function ThemeProvider({ children }: { children: ReactNode }) {
  return (
    <ThemeContext.Provider
      value={{ uiTheme: "default", setUITheme: () => {}, t: defaultText }}
    >
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  return useContext(ThemeContext);
}

export function useThemeText() {
  return useContext(ThemeContext).t;
}

