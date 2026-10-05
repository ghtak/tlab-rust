import { useNavigate } from '@tanstack/react-router'
import { useLogout } from '../hooks/useLogout'
import type { CurrentUser } from '../types'
import { Button } from '../../../components/ui/button'

export function AccountPage({ user }: { user: CurrentUser }) {
  const signOut = useLogout()
  const navigate = useNavigate()

  function handleLogout() {
    signOut.mutate(undefined, {
      onSuccess: () => {
        void navigate({ to: '/login' })
      },
    })
  }

  return (
    <main className="mx-auto flex min-h-screen max-w-3xl flex-col gap-8 p-8">
      <header className="flex items-center justify-between border-b pb-5">
        <span className="text-lg font-semibold">tlab</span>
        <Button variant="outline" onClick={handleLogout} disabled={signOut.isPending}>
          로그아웃
        </Button>
      </header>
      <div>
        <h1 className="text-3xl font-semibold">안녕하세요, {user.name}님</h1>
        <p className="mt-2 text-muted-foreground">{user.email}</p>
        {signOut.isError && <p className="mt-4 text-sm text-destructive">로그아웃하지 못했습니다. 다시 시도해 주세요.</p>}
      </div>
    </main>
  )
}
