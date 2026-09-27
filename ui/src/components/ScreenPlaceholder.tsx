import { useTranslation } from 'react-i18next'

export function ScreenPlaceholder({ labelKey }: { labelKey: string }) {
  const { t } = useTranslation()

  return (
    <div className="p-4">
      <h1 className="mb-2 text-base font-semibold">{t(labelKey)}</h1>
      <p className="text-sm text-[var(--color-text-secondary)]">
        {t('placeholder.screenBody', { screen: t(labelKey) })}
      </p>
    </div>
  )
}
