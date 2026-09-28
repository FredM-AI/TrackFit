import { createFileRoute, redirect } from '@tanstack/react-router'
import { Home } from '@/screens/Home'
import { isFirstLaunchComplete } from '@/lib/api'

export const Route = createFileRoute('/')({
  beforeLoad: async () => {
    // M3-1 : redirige vers l'assistant de premier lancement tant qu'il n'a
    // pas ete complete. Si l'appel IPC echoue (hors d'un contexte Tauri, ex.
    // les tests), on considere l'assistant comme deja fait plutot que de
    // bloquer l'ecran d'accueil.
    let complete: boolean
    try {
      complete = await isFirstLaunchComplete()
    } catch {
      complete = true
    }
    if (!complete) {
      throw redirect({ to: '/setup' })
    }
  },
  component: Home,
})
