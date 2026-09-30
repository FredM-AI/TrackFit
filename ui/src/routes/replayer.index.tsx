import { createFileRoute } from '@tanstack/react-router'
import { ReplayerEmpty } from '@/screens/Replayer'

export const Route = createFileRoute('/replayer/')({
  component: ReplayerEmpty,
})
