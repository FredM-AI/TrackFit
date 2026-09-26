import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import App from './App'

describe('App', () => {
  it('renders the Graphite shell', () => {
    render(<App />)
    expect(screen.getByText('Graphite')).toBeInTheDocument()
  })
})
