import { createFileRoute } from '@tanstack/react-router'
import { Tournaments } from '@/screens/Tournaments'

export const Route = createFileRoute('/tournaments')({
  component: Tournaments,
})
