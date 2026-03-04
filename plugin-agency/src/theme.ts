/**
 * Busbar Agency TUI theme.
 * Change colors here to restyle the whole UI.
 *
 * Valid values: 'black' | 'red' | 'green' | 'yellow' | 'blue' | 'magenta' |
 *               'cyan' | 'white' | 'gray' | 'grey' | '*Bright' variants
 */
export const theme = {
  // ── Brand ──────────────────────────────────────────────────────────────────
  /** Primary brand color — the logo mark and active tab indicator */
  brand:   'blue'        as const,
  /** Secondary accent — selected items, active borders, highlights */
  accent:  'cyan'        as const,
  /** Logo mark character. ☰ = three stacked bars (busbar) */
  logoMark: '☰',
  /** Product name shown in header */
  productName: 'Agency',

  // ── Semantic ────────────────────────────────────────────────────────────────
  success:  'green'      as const,
  warning:  'yellow'     as const,
  error:    'red'        as const,
  muted:    'gray'       as const,
  text:     'white'      as const,

  // ── Graph view colors ───────────────────────────────────────────────────────
  topicEntry:   'white'       as const,
  topicNormal:  'cyan'        as const,
  actionFlow:   'cyanBright'  as const,
  actionApex:   'magenta'     as const,
  actionPrompt: 'greenBright' as const,
  varRead:      'green'       as const,
  varWrite:     'yellow'      as const,
} as const;
