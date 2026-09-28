import { describe, expect, it } from 'vitest'
import i18next from '@/lib/i18n'

// Regression : le plugin `i18next-icu` remplace l'interpolation par defaut
// de i18next (`{{var}}`) par le format ICU MessageFormat (`{var}`). Une cle
// qui utiliserait encore la syntaxe double-accolade s'affiche telle quelle,
// sans substitution (bug reel observe manuellement par Frederic sur
// statusBar.handsToday et setup.handFileCount/importProgress).
describe('i18n interpolation (ICU)', () => {
  it('substitutes a simple numeric variable', () => {
    const text = i18next.t('statusBar.handsToday', { count: 5 })
    expect(text).toContain('5')
    expect(text).not.toContain('{')
  })

  it('substitutes setup.handFileCount', () => {
    const text = i18next.t('setup.handFileCount', { count: 42 })
    expect(text).toContain('42')
    expect(text).not.toContain('{')
  })

  it('substitutes multiple variables in setup.importProgress', () => {
    const text = i18next.t('setup.importProgress', { done: 2, total: 5, hands: 117 })
    expect(text).toContain('2')
    expect(text).toContain('5')
    expect(text).toContain('117')
    expect(text).not.toContain('{')
  })

  it('substitutes setup.doneSummary', () => {
    const text = i18next.t('setup.doneSummary', { hands: 328, tournaments: 2 })
    expect(text).toContain('328')
    expect(text).toContain('2')
    expect(text).not.toContain('{')
  })

  it('does not choke on an apostrophe elsewhere in the same string (French)', () => {
    // "Mains aujourd'hui" contient une apostrophe : ICU MessageFormat traite
    // l'apostrophe comme un caractere d'echappement special, a verifier.
    const text = i18next.t('statusBar.handsToday', { count: 3 })
    expect(text).toBe("Mains aujourd'hui : 3")
  })

  it('renders setup.noAccountsFound (contains an apostrophe) unchanged', () => {
    const text = i18next.t('setup.noAccountsFound')
    expect(text).toContain("l'import")
    expect(text).not.toContain('{')
  })
})
