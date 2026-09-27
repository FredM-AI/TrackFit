import ICU from 'i18next-icu'
import { initReactI18next } from 'react-i18next'
import i18next from 'i18next'
import en from '@/locales/en/common.json'
import fr from '@/locales/fr/common.json'

void i18next
  .use(ICU)
  .use(initReactI18next)
  .init({
    resources: {
      fr: { common: fr },
      en: { common: en },
    },
    lng: 'fr',
    fallbackLng: 'fr',
    defaultNS: 'common',
    interpolation: { escapeValue: false },
  })

export default i18next
