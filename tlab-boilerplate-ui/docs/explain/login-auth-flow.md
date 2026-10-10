# 로그인 화면과 로그인 상태 관리

이 문서는 UI 프로젝트의 로그인 구현을 React에 익숙하지 않은 개발자도 따라갈 수 있도록 설명한다.

## 파일 구성

```text
src/
  api.ts                 공통 HTTP 클라이언트
  features/
    auth/
      types.ts            현재 사용자 타입
      pages/
        LoginPage.tsx     로그인 폼
        AccountPage.tsx   사용자 정보와 로그아웃 화면
      services/
        api.ts           로그인·로그아웃·현재 사용자 API
        queries.ts       현재 사용자 조회 설정
      hooks/
        useLogin.ts      로그인 요청과 사용자 캐시 갱신
        useLogout.ts     로그아웃 요청과 사용자 캐시 삭제
  routes/
    login.tsx            /login 경로와 진입 검사
    index.tsx            / 경로와 진입 검사
```

인증에 속하는 코드는 `features/auth`에 모은다. `types.ts`는 인증 모듈에서 공유하는 현재 사용자 타입, `pages`는 화면, `services`는 서버 통신과 조회 설정, `hooks`는 로그인·로그아웃 요청 상태를 맡는다. `routes`에는 URL과 화면을 연결하는 코드만 둔다.

## 전체 흐름

```text
로그인 화면에서 이메일·비밀번호 입력
  → POST /api/v1/auth/login
  → 서버가 인증 쿠키 발급
  → UI가 GET /api/v1/auth/me로 사용자 확인
  → 사용자 정보가 있으면 홈 화면 표시

페이지 새로고침
  → UI가 /me를 다시 조회
  → 브라우저에 유효한 인증 쿠키가 있으면 로그인 상태 복원

로그아웃
  → POST /api/v1/auth/logout
  → 서버가 인증 쿠키 삭제
  → UI가 사용자 정보 캐시를 비우고 로그인 화면으로 이동
```

로그인 여부의 기준은 UI의 `true`/`false` 변수나 저장된 토큰이 아니라 서버의 `/me` 응답이다. 브라우저가 인증 쿠키를 요청에 실어 보내며, UI는 쿠키 값을 직접 읽거나 저장하지 않는다.

## React에서 쓰는 두 종류의 상태

- **입력 상태:** 로그인 화면의 이메일과 비밀번호. React의 `useState`가 현재 입력값을 기억한다.
- **서버 상태:** 현재 사용자 정보. TanStack Query가 `/me` 응답을 조회하고 잠시 보관한다.

이 둘을 구분하면 새로고침 시 로그인 상태를 복원하는 이유가 분명해진다. `useState` 값은 페이지를 새로 열면 사라지지만, 인증 쿠키가 남아 있으면 `/me`를 다시 조회해 사용자를 확인할 수 있다.

## API 요청 코드

[`src/api.ts`](../../src/api.ts)는 API 요청에 쓰는 `ky` 객체를 만든다.

```ts
export const api = ky.create({ prefix: '/api/v1', retry: 0 })
```

따라서 `api.get('auth/me')`는 `/api/v1/auth/me`로 요청한다. `ky`는 브라우저의 HTTP 요청을 다루는 라이브러리다.

[`src/features/auth/types.ts`](../../src/features/auth/types.ts)의 `CurrentUser`는 `/me` 응답의 사용자 구조를 정의한다. [`src/features/auth/services/api.ts`](../../src/features/auth/services/api.ts)의 `getCurrentUser()`는 그 응답을 해석한다.

```ts
async function getCurrentUser(): Promise<CurrentUser | null> {
  try {
    const response = await api.get('auth/me').json<{ data: CurrentUser }>()
    return response.data
  } catch (error) {
    if (error instanceof HTTPError && error.response.status === 401) return null
    throw error
  }
}
```

성공하면 사용자 정보를 반환한다. `401`은 인증되지 않았다는 뜻이므로 `null`을 반환한다. 서버 장애처럼 다른 오류는 그대로 전달한다. 그런 오류까지 로그아웃으로 취급하면 일시적인 장애에 화면이 잘못 반응할 수 있기 때문이다.

[`src/features/auth/services/queries.ts`](../../src/features/auth/services/queries.ts)의 `currentUserQuery`는 이 요청을 TanStack Query에 등록한다.

```ts
export const currentUserQuery = queryOptions({
  queryKey: ['currentUser'],
  queryFn: getCurrentUser,
  retry: false,
})
```

`queryKey`는 조회 결과에 붙인 이름이고, `queryFn`은 실제 조회 함수다. `api.ts`의 `login()`과 `logout()` 함수는 각각 서버의 로그인·로그아웃 API를 호출한다.

## 라우터가 화면을 보여주기 전에 확인하는 것

[`src/routes/index.tsx`](../../src/routes/index.tsx)의 홈 라우트는 `beforeLoad`에서 사용자를 확인한다.

```ts
const user = await context.queryClient.query(currentUserQuery)
if (!user) throw redirect({ to: '/login' })
return { user }
```

`beforeLoad`는 화면 컴포넌트를 보여주기 전에 실행된다. 사용자가 없으면 로그인 화면으로 이동하고, 있으면 `user`를 라우트 컴포넌트에 전달한다. 이 컴포넌트는 `Route.useRouteContext()`로 값을 받아 [`AccountPage`](../../src/features/auth/pages/AccountPage.tsx)에 넘긴다. `AccountPage`가 이름과 이메일을 표시한다.

반대로 [`src/routes/login.tsx`](../../src/routes/login.tsx)는 이미 로그인한 사용자가 `/login`에 들어오면 홈으로 보낸다.

`beforeLoad`는 React 컴포넌트 밖에서 실행되므로 React 훅인 `useQueryClient()`를 호출할 수 없다. 그래서 [`src/main.tsx`](../../src/main.tsx)에서 `QueryClient`를 만들고, [`src/router.tsx`](../../src/router.tsx)의 라우터 `context`로 넘긴다. [`src/routes/__root.tsx`](../../src/routes/__root.tsx)는 그 `context`의 타입을 선언한다. 컴포넌트 안에서는 `QueryClientProvider`가 제공하는 같은 객체를 `useQueryClient()`로 가져온다.

현재 보호되는 화면은 홈(`/`)이다. 보호할 화면이 늘어나면 해당 라우트에도 인증 확인을 적용해야 한다.

## 로그인 폼의 React 코드

[`src/features/auth/pages/LoginPage.tsx`](../../src/features/auth/pages/LoginPage.tsx)의 `LoginPage` 함수는 화면을 그리는 **컴포넌트**다. 이메일과 비밀번호 입력값은 각각 `useState`로 관리한다.

```ts
const [email, setEmail] = useState('')
const [password, setPassword] = useState('')
```

입력창의 `value`는 현재 상태이고, `onChange`는 사용자가 입력할 때 상태를 바꾼다. 상태가 바뀌면 React가 컴포넌트를 다시 실행해 화면에 새 값을 반영한다.

폼을 제출하면 `submit()`이 실행된다. `event.preventDefault()`는 브라우저의 기본 폼 제출에 따른 페이지 새로고침을 막고, `signIn.mutate({ email, password })`가 로그인 요청을 시작한다. 성공 후 화면 이동은 `LoginPage`가 결정한다.

```ts
const signIn = useLogin()

signIn.mutate({ email, password }, {
  onSuccess: () => {
    void navigate({ to: '/' })
  },
})

// useLogin 내부
return useMutation({
  mutationFn: ({ email, password }) => login(email, password),
  onSuccess: async () => {
    await queryClient.invalidateQueries({ queryKey: currentUserQuery.queryKey })
  },
})
```

[`useLogin`](../../src/features/auth/hooks/useLogin.ts)은 로그인 요청과 사용자 캐시 갱신을 묶은 훅이다. 내부의 `useMutation`이 요청 진행 상태를 관리한다. `signIn.isPending`일 때 버튼을 비활성화하고, `signIn.isError`일 때 오류 문구를 보여준다. 비밀번호 오류에 해당하는 `401`은 별도 문구로 처리한다.

로그인이 성공하면 `currentUser` 조회 결과를 무효화하고 홈으로 이동한다. 홈의 `beforeLoad`가 `/me`를 다시 조회해 실제 사용자 정보를 확인한다.

## 로그아웃 코드

[`src/features/auth/pages/AccountPage.tsx`](../../src/features/auth/pages/AccountPage.tsx)는 [`useLogout`](../../src/features/auth/hooks/useLogout.ts)을 사용한다. 이 훅은 `useMutation`으로 서버의 로그아웃 API를 호출하고, 성공하면 `queryClient.clear()`로 보관 중인 조회 데이터를 비운다. `/login` 이동은 `AccountPage`가 결정한다. 서버는 로그아웃 응답에서 인증 쿠키를 삭제한다. 요청이 실패하면 화면에 오류 문구를 표시한다.

## 백엔드의 `/me` 변경

기존 `/me`는 `access:manage` 권한을 요구하고 JWT 내용을 반환했다. 일반 사용자는 로그인해도 권한이 없어 `403`을 받으므로 UI의 로그인 상태 확인에 사용할 수 없었다.

이제 [`auth/route.rs`](../../../tlab-boilerplate/src/auth/route.rs)의 `/me`는 인증된 사용자의 ID로 DB를 조회하고 `id`, `name`, `email`, `status`를 반환한다. 사용자 조회 코드는 [`user_repository.rs`](../../../tlab-boilerplate/src/auth/repository/user_repository.rs)에 추가했다. 응답에는 `Cache-Control: no-store`를 붙여 브라우저가 사용자 정보를 HTTP 캐시에 보관하지 않도록 했다.

화면에서 이동을 제한하는 것은 사용성 처리다. 실제 데이터 접근 권한은 계속 백엔드 API에서 검사해야 한다.

## 개발 환경의 API 경로

[`vite.config.ts`](../../vite.config.ts)는 개발 서버의 `/api` 요청을 `https://localhost:13000`으로 전달한다. UI 코드는 항상 `/api/v1/...` 경로를 사용한다. `secure: false`는 개발 백엔드의 자체 서명 TLS 인증서를 프록시가 검증하지 않도록 하는 설정이다.

## 검증

- TypeScript 타입 검사와 UI 빌드가 통과했다.
- 백엔드 인증 라우트 테스트 6개가 통과했다. 여기에는 관리 권한이 없는 사용자의 `/me` 조회 테스트가 포함된다.
- `npm run check`는 기존 Biome 설정 버전과 프로젝트 전반의 서식 불일치로 실패했다.
