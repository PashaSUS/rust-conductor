import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useThemeText } from "@/components/ThemeContext";

export function CopyButton({ value, className }: { value: string; className?: string }) {
  const t = useThemeText();
  const [copied, setCopied] = useState(false);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(value).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <Button variant="ghost" size="icon" className={`h-5 w-5 shrink-0 ${className ?? ""}`} onClick={handleCopy} title={t.copyToClipboard}>
      {copied ? <Check className="h-3 w-3 text-emerald-500" /> : <Copy className="h-3 w-3 text-muted-foreground" />}
    </Button>
  );
}
