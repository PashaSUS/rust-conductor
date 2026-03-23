export type { UITheme, ThemeText } from "./types";
import type { UITheme, ThemeText } from "./types";

import { defaultText } from "./default";
import { warcraftText } from "./warcraft";
import { cyberpunkText } from "./cyberpunk";
import { forestText } from "./forest";
import { oceanText } from "./ocean";
import { pokemonText } from "./pokemon";
import { yugiohText } from "./yugioh";
import { chucknorrisText } from "./chucknorris";
import { lotrText } from "./lotr";

export { defaultText };

export const themeTextMap: Record<UITheme, ThemeText> = {
  default: defaultText,
  warcraft: warcraftText,
  cyberpunk: cyberpunkText,
  forest: forestText,
  ocean: oceanText,
  pokemon: pokemonText,
  yugioh: yugiohText,
  chucknorris: chucknorrisText,
  lotr: lotrText,
};
