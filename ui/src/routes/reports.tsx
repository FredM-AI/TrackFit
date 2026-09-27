import { createFileRoute } from '@tanstack/react-router'
import { Reports } from '@/screens/Reports'

export const Route = createFileRoute('/reports')({
  component: Reports,
})
