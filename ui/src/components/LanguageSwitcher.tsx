import { useTranslation } from 'react-i18next'

const LANGUAGES = ['fr', 'en'] as const

export function LanguageSwitcher() {
  const { t, i18n } = useTranslation()

  return (
    <label className="flex items-center gap-2 text-xs text-[var(--color-text-secondary)]">
      <span className="sr-only">{t('language.label')}</span>
      <select
        value={i18n.language}
        onChange={(event) => {
          void i18n.changeLanguage(event.target.value)
        }}
        className="rounded border border-[var(--color-border)] bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[var(--color-text-primary)]"
      >
        {LANGUAGES.map((lng) => (
          <option key={lng} value={lng}>
            {t(`language.${lng}`)}
          </option>
        ))}
      </select>
    </label>
  )
}
