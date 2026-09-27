import { createFileRoute } from '@tanstack/react-router'
import { Leaks } from '@/screens/Leaks'

export const Route = createFileRoute('/leaks')({
  component: Leaks,
})
