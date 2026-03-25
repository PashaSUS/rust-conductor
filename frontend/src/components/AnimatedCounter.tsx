import { useEffect, useRef, useState } from "react";

interface AnimatedCounterProps {
  value: number;
  duration?: number;
  className?: string;
}

/**
 * Smoothly animates a number change with an ease-out curve.
 * Used on Dashboard stat cards for a polished feel.
 */
export function AnimatedCounter({ value, duration = 600, className }: AnimatedCounterProps) {
  const [display, setDisplay] = useState(0);
  const prevValue = useRef(0);
  const raf = useRef<number | null>(null);

  useEffect(() => {
    const from = prevValue.current;
    const to = value;
    const start = performance.now();

    const tick = (now: number) => {
      const elapsed = now - start;
      const progress = Math.min(elapsed / duration, 1);
      // ease-out cubic
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = Math.round(from + (to - from) * eased);
      setDisplay(current);

      if (progress < 1) {
        raf.current = requestAnimationFrame(tick);
      } else {
        prevValue.current = to;
      }
    };

    raf.current = requestAnimationFrame(tick);
    return () => {
      if (raf.current) cancelAnimationFrame(raf.current);
    };
  }, [value, duration]);

  return <span className={className}>{display.toLocaleString()}</span>;
}
