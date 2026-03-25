import { useState, useEffect, useRef } from "react";
import { Palette } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useTheme, useThemeText, type UITheme } from "@/components/ThemeContext";

const themes: { id: UITheme; label: string; description: string; preview: string[] }[] = [
  {
    id: "default",
    label: "Default",
    description: "Clean modern interface",
    preview: ["#f8f8fa", "#1a1a2e", "#6366f1", "#e2e2e8"],
  },
  {
    id: "warcraft",
    label: "Warcraft",
    description: "Gold & parchment medieval feel",
    preview: ["#1a1412", "#c9a44a", "#e8d5b0", "#5c4a32"],
  },
  {
    id: "cyberpunk",
    label: "Cyberpunk",
    description: "Neon tech-noir futuristic",
    preview: ["#0a0a12", "#00f0ff", "#ff2266", "#2a2a50"],
  },
  {
    id: "forest",
    label: "Forest",
    description: "Earthy greens & wood tones",
    preview: ["#161e14", "#5ea84e", "#d4e0c8", "#3a5035"],
  },
  {
    id: "ocean",
    label: "Ocean",
    description: "Deep blues & aqua accents",
    preview: ["#0c1824", "#38a8d0", "#c8dce8", "#284060"],
  },
  {
    id: "pokemon",
    label: "PokÃ©mon",
    description: "Gotta orchestrate 'em all!",
    preview: ["#1a1020", "#ef4444", "#eab308", "#3b82f6"],
  },
  {
    id: "chucknorris",
    label: "Chuck Norris",
    description: "Bold red/black action hero",
    preview: ["#141010", "#ef4444", "#a0a0a0", "#504040"],
  },
  {
    id: "lotr",
    label: "Lord of the Rings",
    description: "Elvish gold & mithril, ancient forest",
    preview: ["#0e1410", "#b09860", "#708898", "#3a5040"],
  },
];

export function ThemeSwitcher() {
  const { uiTheme, setUITheme } = useTheme();
  const t = useThemeText();
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  // Close panel on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (
        panelRef.current &&
        !panelRef.current.contains(e.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open]);

  return (
    <div className="relative">
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            ref={buttonRef}
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => setOpen((o) => !o)}
          >
            <Palette className="h-4 w-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="right">
          <p className="text-xs">{t.uiTheme}: {themes.find((th) => th.id === uiTheme)?.label}</p>
        </TooltipContent>
      </Tooltip>

      {open && (
        <div
          ref={panelRef}
          className="absolute left-full bottom-0 ml-2 z-50 w-64 rounded-lg border bg-popover p-3 shadow-lg"
        >
          <p className="text-xs font-semibold text-popover-foreground mb-2">{t.uiTheme}</p>
          <div className="space-y-1">
            {themes.map((theme) => (
              <button
                key={theme.id}
                onClick={() => {
                  setUITheme(theme.id);
                  setOpen(false);
                }}
                className={cn(
                  "w-full flex items-center gap-3 rounded-md px-2 py-2 text-left text-sm transition-colors",
                  uiTheme === theme.id
                    ? "bg-accent text-accent-foreground"
                    : "text-popover-foreground hover:bg-accent"
                )}
              >
                {/* Color preview dots */}
                <div className="flex gap-0.5 shrink-0">
                  {theme.preview.map((color, i) => (
                    <div
                      key={i}
                      className="w-3 h-3 rounded-full border border-border"
                      style={{ backgroundColor: color }}
                    />
                  ))}
                </div>
                <div className="min-w-0">
                  <div className="font-medium truncate">{theme.label}</div>
                  <div className="text-[10px] text-muted-foreground truncate">
                    {theme.description}
                  </div>
                </div>
                {uiTheme === theme.id && (
                  <div className="ml-auto shrink-0 h-2 w-2 rounded-full bg-primary" />
                )}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
