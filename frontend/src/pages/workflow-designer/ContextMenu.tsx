import { useRef, useEffect } from "react";
import type { ContextMenuItem } from "./types";

export function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: ContextMenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as HTMLElement)) onClose();
    };
    const keyHandler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("click", handler);
    document.addEventListener("keydown", keyHandler);
    return () => { document.removeEventListener("click", handler); document.removeEventListener("keydown", keyHandler); };
  }, [onClose]);

  return (
    <div
      ref={ref}
      style={{ position: "fixed", top: y, left: x, zIndex: 9999 }}
      className="min-w-45 rounded-lg border bg-popover text-popover-foreground shadow-lg py-1 animate-in fade-in-0 zoom-in-95"
    >
      {items.map((item, i) =>
        item.separator ? (
          <div key={i} className="h-px bg-border my-1" />
        ) : (
          <button
            key={i}
            onClick={() => { item.onClick(); onClose(); }}
            disabled={item.disabled}
            className={`w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left transition-colors
              ${item.danger ? "text-destructive hover:bg-destructive/10" : "hover:bg-accent"}
              ${item.disabled ? "opacity-40 cursor-not-allowed" : "cursor-pointer"}`}
          >
            {item.icon && <span className="w-4 h-4 flex items-center justify-center shrink-0">{item.icon}</span>}
            {item.label}
          </button>
        )
      )}
    </div>
  );
}
