import { createRouter, RouterProvider } from '@tanstack/react-router'
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import '@/lib/i18n'
import { routeTree } from '@/routeTree.gen'

describe('AppShell', () => {
  it('renders the home screen with the sidebar navigation', async () => {
    const router = createRouter({ routeTree })
    render(<RouterProvider router={router} />)

    expect(await screen.findAllByText('Graphite')).not.toHaveLength(0)
    expect(await screen.findByText('Résultats')).toBeInTheDocument()
    expect(await screen.findByText('Replayer')).toBeInTheDocument()
  })
})
