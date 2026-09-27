import { createFileRoute } from '@tanstack/react-router'
import { Logs } from '@/screens/Logs'

export const Route = createFileRoute('/logs')({
  component: Logs,
})
