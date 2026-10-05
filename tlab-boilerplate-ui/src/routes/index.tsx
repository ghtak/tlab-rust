import { createFileRoute, redirect } from '@tanstack/react-router'
import { AccountPage } from '../features/auth/pages/AccountPage'
import { currentUserQuery } from '../features/auth/services/queries'

export const Route = createFileRoute('/')({
  beforeLoad: async ({ context }) => {
    const user = await context.queryClient.query(currentUserQuery)
    if (!user) throw redirect({ to: '/login' })
    return { user }
  },
  component: Home,
})

function Home() {
  const { user } = Route.useRouteContext()
  return <AccountPage user={user} />
}
