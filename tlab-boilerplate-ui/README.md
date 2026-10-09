# tlab-boilerplate-ui

`tlab-boilerplate` API를 사용하는 React UI다. 새 화면은 기존 `src/features`와 `src/routes`의 흐름을 참고해 필요한 부분만 추가한다.

## 실행

```sh
npm install
npm run dev
```

개발 서버는 3000 포트에서 실행하며 `/api` 요청을 `https://localhost:13000`으로 전달한다. API 서버 설정은 [`../tlab-boilerplate/config.yml`](../tlab-boilerplate/config.yml), 프록시 설정은 `vite.config.ts`에 있다.

## 새 UI 기능 연결 지점

1. API 타입이 필요하면 `src/features/<기능명>/types.ts`에 둔다. 기존 타입에 필드를 더해야 하면 확장 타입으로 표현한다.
2. API 요청이 있으면 기능의 `services/api.ts`에서 `src/api.ts`의 `api` 클라이언트를 사용하고, 조회 캐시가 필요하면 `services/queries.ts`에 TanStack Query 옵션을 둔다. 기존 `src/features/access/services`가 예시다.
3. 화면은 기능 폴더의 `pages`와 필요한 `components`에 둔다. `src/routes`에는 화면을 연결하는 파일 라우트를 추가한다. `src/routes/_app.permissions.tsx`와 `src/features/access/pages/PermissionPage.tsx`가 예시다.
4. 앱 메뉴에 노출해야 하면 `src/features/app-shell/AppLayout.tsx`의 메뉴 항목을 추가한다. `_app` 하위 라우트는 `src/routes/_app.tsx`에서 로그인 상태를 확인한다. 화면별 권한 정책은 각 기능 요구 사항에 맞춰 정한다.
5. 데이터 변경 후 관련 조회를 갱신해야 하면 해당 query key를 무효화한다. `src/features/access/components/CreatePermissionDialog.tsx`가 예시다.

인증 흐름은 [`docs/explain/login-auth-flow.md`](docs/explain/login-auth-flow.md)를 참고한다. 백엔드 기능의 계층과 연결 지점은 [`../tlab-boilerplate/docs/feature-structure.md`](../tlab-boilerplate/docs/feature-structure.md)에 정리되어 있다.

## UI 프로젝트 생성 기록

현재 UI 프로젝트를 생성할 때 사용한 명령이다. 기존 프로젝트를 실행할 때 다시 실행할 필요는 없다.

```sh
npx @tanstack/cli@latest create tlab-boilerplate-ui --router-only --framework react --package-manager npm --toolchain biome --no-examples --no-git

npm i @tanstack/react-query
npm i -D @tanstack/eslint-plugin-query
npm i ky
npm i zod
npm i tailwindcss @tailwindcss/vite
npx shadcn@latest init -t vite
```
