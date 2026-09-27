import {
  AlertTriangle,
  FileBarChart,
  Home,
  type LucideIcon,
  Play,
  Rows3,
  ScrollText,
  Settings,
  TrendingUp,
  Trophy,
} from 'lucide-react'

export interface NavItem {
  to: string
  labelKey: string
  icon: LucideIcon
}

export const navItems: NavItem[] = [
  { to: '/', labelKey: 'nav.home', icon: Home },
  { to: '/results', labelKey: 'nav.results', icon: TrendingUp },
  { to: '/tournaments', labelKey: 'nav.tournaments', icon: Trophy },
  { to: '/hands', labelKey: 'nav.hands', icon: Rows3 },
  { to: '/reports', labelKey: 'nav.reports', icon: FileBarChart },
  { to: '/leaks', labelKey: 'nav.leaks', icon: AlertTriangle },
  { to: '/replayer', labelKey: 'nav.replayer', icon: Play },
  { to: '/logs', labelKey: 'nav.logs', icon: ScrollText },
  { to: '/settings', labelKey: 'nav.settings', icon: Settings },
]
