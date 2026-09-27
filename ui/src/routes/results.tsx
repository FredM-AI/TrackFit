import { createFileRoute } from '@tanstack/react-router'
import { Results } from '@/screens/Results'

export const Route = createFileRoute('/results')({
  component: Results,
})
