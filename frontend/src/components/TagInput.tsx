import { useState, useRef, type KeyboardEvent } from "react";
import { X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { useThemeText } from "@/components/ThemeContext";

interface TagInputProps {
  tags: string[];
  onChange: (tags: string[]) => void;
  placeholder?: string;
  id?: string;
}

export function TagInput({ tags, onChange, placeholder, id }: TagInputProps) {
  const t = useThemeText();
  const resolvedPlaceholder = placeholder ?? t.typeAndPressEnter;
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const addTag = (value: string) => {
    const trimmed = value.trim();
    if (trimmed && !tags.includes(trimmed)) {
      onChange([...tags, trimmed]);
    }
    setInput("");
  };

  const removeTag = (index: number) => {
    onChange(tags.filter((_, i) => i !== index));
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if ((e.key === "Enter" || e.key === ",") && input.trim()) {
      e.preventDefault();
      addTag(input);
    } else if (e.key === "Backspace" && !input && tags.length > 0) {
      removeTag(tags.length - 1);
    }
  };

  return (
    <div
      className="flex flex-wrap items-center gap-1.5 rounded-md border border-input bg-background px-3 py-1.5 text-sm shadow-sm ring-offset-background focus-within:ring-1 focus-within:ring-ring min-h-9 cursor-text"
      onClick={() => inputRef.current?.focus()}
    >
      {tags.map((tag, i) => (
        <Badge key={tag} variant="secondary" className="gap-1 px-2 py-0.5 text-xs font-normal">
          {tag}
          <button
            type="button"
            onClick={(e) => { e.stopPropagation(); removeTag(i); }}
            className="ml-0.5 rounded-full hover:bg-muted p-0.5"
          >
            <X className="h-3 w-3" />
          </button>
        </Badge>
      ))}
      <input
        ref={inputRef}
        id={id}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={() => { if (input.trim()) addTag(input); }}
        placeholder={tags.length === 0 ? resolvedPlaceholder : ""}
        className="flex-1 min-w-20 bg-transparent outline-none placeholder:text-muted-foreground text-sm"
      />
    </div>
  );
}
