import { useState, type SubmitEvent } from 'react'
import { useLocation, useNavigate } from '@tanstack/react-router'
import { HTTPError } from 'ky'
import { useLogin } from '../hooks/useLogin'
import { Button, buttonVariants } from '../../../components/ui/button'

export function LoginPage() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const signIn = useLogin()
  const navigate = useNavigate()
  const googleError = useLocation({ select: (location) => new URLSearchParams(location.searchStr).get('error') })

  function submit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault()
    signIn.mutate({ email, password }, {
      onSuccess: () => {
        void navigate({ to: '/' })
      },
    })
  }

  const invalidCredentials = signIn.error instanceof HTTPError && signIn.error.response.status === 401
  const googleErrorMessage = googleError === 'google_cancelled'
    ? 'Google 로그인이 취소되었습니다.'
    : googleError === 'account_exists'
      ? '이미 같은 이메일로 가입된 계정이 있습니다. 기존 계정으로 로그인해 주세요.'
      : googleError
        ? 'Google로 로그인하지 못했습니다. 다시 시도해 주세요.'
        : null

  return (
    <main className="flex min-h-screen items-center justify-center bg-muted/40 px-4 py-12">
      <div className="w-full max-w-sm rounded-xl border bg-card p-8 shadow-sm">
        <div className="mb-8">
          <p className="text-sm font-semibold text-muted-foreground">tlab</p>
          <h1 className="mt-3 text-2xl font-semibold">로그인</h1>
          <p className="mt-2 text-sm text-muted-foreground">이메일과 비밀번호를 입력해 주세요.</p>
        </div>
        <form className="space-y-5" onSubmit={submit}>
          <div className="space-y-2">
            <label className="block text-sm font-medium" htmlFor="email">이메일</label>
            <input id="email" type="email" autoComplete="username" required value={email} onChange={(event) => setEmail(event.target.value)} className="w-full rounded-lg border bg-background px-3 py-2 outline-none focus-visible:ring-2 focus-visible:ring-ring" />
          </div>
          <div className="space-y-2">
            <label className="block text-sm font-medium" htmlFor="password">비밀번호</label>
            <input id="password" type="password" autoComplete="current-password" required value={password} onChange={(event) => setPassword(event.target.value)} className="w-full rounded-lg border bg-background px-3 py-2 outline-none focus-visible:ring-2 focus-visible:ring-ring" />
          </div>
          {signIn.isError && (
            <p role="alert" className="text-sm text-destructive">
              {invalidCredentials ? '이메일 또는 비밀번호를 확인해 주세요.' : '로그인하지 못했습니다. 잠시 후 다시 시도해 주세요.'}
            </p>
          )}
          <Button type="submit" size="lg" className="w-full" disabled={signIn.isPending}>
            {signIn.isPending ? '로그인 중...' : '로그인'}
          </Button>
        </form>
        <div className="my-6 flex items-center gap-3 text-xs text-muted-foreground">
          <span className="h-px flex-1 bg-border" />
          또는
          <span className="h-px flex-1 bg-border" />
        </div>
        <a href="/api/v1/auth/google/auth_url" className={buttonVariants({ variant: 'outline', size: 'lg', className: 'w-full' })}>
          Google로 계속하기
        </a>
        {googleErrorMessage && <p role="alert" className="mt-4 text-sm text-destructive">{googleErrorMessage}</p>}
      </div>
    </main>
  )
}
