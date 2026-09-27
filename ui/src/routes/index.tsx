import { createFileRoute } from '@tanstack/react-router'
import { Home } from '@/screens/Home'

export const Route = createFileRoute('/')({
  component: Home,
})
