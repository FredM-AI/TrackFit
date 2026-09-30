import { createFileRoute } from '@tanstack/react-router'
import { TournamentDetail } from '@/screens/TournamentDetail'

export const Route = createFileRoute('/tournaments/$tournamentId')({
  component: TournamentDetail,
})
