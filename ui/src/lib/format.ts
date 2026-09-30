/** Formatage des nombres affiches (PRD §16 : "chiffres en police tabulaire",
 * pas de logique de style ici — seulement la valeur textuelle). Locale du
 * navigateur (`undefined`), meme convention que `Intl.DateTimeFormat` dans
 * `AppShell.tsx` : pas liee au choix fr/en de l'app. */

const DASH = '—'

export function formatCents(cents: number | null | undefined): string {
  if (cents == null) return DASH
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'EUR',
    signDisplay: 'exceptZero',
  }).format(cents / 100)
}

export function formatCentsPerHour(centsPerHour: number | null | undefined): string {
  if (centsPerHour == null) return DASH
  return `${formatCents(centsPerHour)}/h`
}

export function formatPercent(ratio: number | null | undefined): string {
  if (ratio == null) return DASH
  return new Intl.NumberFormat(undefined, {
    style: 'percent',
    maximumFractionDigits: 1,
    signDisplay: 'exceptZero',
  }).format(ratio)
}

export function formatBb(bb: number | null | undefined): string {
  if (bb == null) return DASH
  return `${new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 1,
    signDisplay: 'exceptZero',
  }).format(bb)} bb`
}

export function formatCount(n: number | null | undefined): string {
  if (n == null) return DASH
  return new Intl.NumberFormat(undefined).format(n)
}

export function formatDate(epochMs: number | null | undefined): string {
  if (epochMs == null) return DASH
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
    new Date(epochMs),
  )
}

export function formatDurationMs(ms: number): string {
  const totalMinutes = Math.round(ms / 60_000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  return `${hours}h${String(minutes).padStart(2, '0')}`
}

/** Classe de couleur signee (D15) : les tokens `--color-positive/negative`
 * valent la meme couleur monochrome par defaut et ne divergent qu'en mode
 * "couleurs semantiques" optionnel — le signe reste toujours visible via
 * `signDisplay: 'exceptZero'` ci-dessus, jamais uniquement via la couleur. */
export function signedValueClassName(value: number): string {
  if (value > 0) return 'text-[var(--color-positive)] font-semibold'
  if (value < 0) return 'text-[var(--color-negative)] font-semibold'
  return ''
}
