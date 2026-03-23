import { createContext, useContext, useState, useEffect, type ReactNode } from "react";
import { themeTextMap, defaultText } from "./themes";
export type { UITheme, ThemeText } from "./themes";
import type { UITheme, ThemeText } from "./themes";

/* ------------------------------------------------------------------ */
/*  Context                                                            */
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

function getStoredUITheme(): UITheme {
  const stored = localStorage.getItem("ui-theme") as UITheme | null;
  return stored && stored in themeTextMap ? stored : "default";
}

function applyUIThemeClass(theme: UITheme) {
  const root = document.documentElement;
  root.classList.remove("theme-warcraft", "theme-cyberpunk", "theme-forest", "theme-ocean", "theme-pokemon", "theme-yugioh", "theme-chucknorris", "theme-lotr");
  if (theme !== "default") {
    root.classList.add(`theme-${theme}`);
  }
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [uiTheme, setUIThemeState] = useState<UITheme>(getStoredUITheme);

  const setUITheme = (theme: UITheme) => {
    setUIThemeState(theme);
    localStorage.setItem("ui-theme", theme);
    applyUIThemeClass(theme);
  };

  useEffect(() => {
    applyUIThemeClass(uiTheme);
  }, [uiTheme]);

  const t = themeTextMap[uiTheme];

  return (
    <ThemeContext.Provider value={{ uiTheme, setUITheme, t }}>
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
