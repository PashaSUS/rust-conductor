import { useEffect, useState } from "react";

export interface ChartColors {
  completed: string;
  running: string;
  failed: string;
  scheduled: string;
  timedOut: string;
  paused: string;
  grid: string;
  text: string;
  axis: string;
  tooltip: { bg: string; border: string };
}

const lightColors: ChartColors = {
  completed: "#10b981",
  running: "#3b82f6",
  failed: "#ef4444",
  scheduled: "#f59e0b",
  timedOut: "#f97316",
  paused: "#8b5cf6",
  grid: "#e5e7eb",
  text: "#6b7280",
  axis: "#9ca3af",
  tooltip: { bg: "#ffffff", border: "#e5e7eb" },
};

const darkColors: ChartColors = {
  completed: "#34d399",
  running: "#60a5fa",
  failed: "#f87171",
  scheduled: "#fbbf24",
  timedOut: "#fb923c",
  paused: "#a78bfa",
  grid: "#374151",
  text: "#d1d5db",
  axis: "#6b7280",
  tooltip: { bg: "#1f2937", border: "#374151" },
};

function isDarkMode(): boolean {
  return document.documentElement.classList.contains("dark");
}

export function useChartColors(): ChartColors {
  const [dark, setDark] = useState(isDarkMode);

  useEffect(() => {
    const observer = new MutationObserver(() => setDark(isDarkMode()));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  return dark ? darkColors : lightColors;
}
