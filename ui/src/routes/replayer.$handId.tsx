import { createFileRoute } from '@tanstack/react-router'
import { Replayer } from '@/screens/Replayer'

export const Route = createFileRoute('/replayer/$handId')({
  component: Replayer,
})
