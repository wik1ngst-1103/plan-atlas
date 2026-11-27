const COLORS = ['#4c6ef5', '#12b886', '#f59f00', '#e64980', '#7950f2', '#15aabf'];

/** Stable colour for a project name (same everywhere: side nav, headings, cards). */
export const projectColor = (name: string) =>
  COLORS[[...name].reduce((h, c) => h + c.charCodeAt(0), 0) % COLORS.length];
