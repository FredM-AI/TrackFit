import { createFileRoute } from '@tanstack/react-router'
import { Hands } from '@/screens/Hands'

export const Route = createFileRoute('/hands')({
  component: Hands,
})
