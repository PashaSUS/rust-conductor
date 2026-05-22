export type { UITheme, ThemeText } from "./types";
import type { UITheme, ThemeText } from "./types";

import { defaultText } from "./default";

export { defaultText };

export const themeTextMap: Record<UITheme, ThemeText> = {
  default: defaultText,
};
