import { createFileRoute } from '@tanstack/react-router'
import { ReplayerLayout } from '@/screens/Replayer'

export const Route = createFileRoute('/replayer')({
  component: ReplayerLayout,
})
