# tlab-rust

## 구성

- `tlab`: DB, HTTP, 인증 관련 공통 라이브러리
- `tlab-boilerplate`: Axum API 서버와 인증·권한 기능. 새 백엔드 기능은 [기능 구조 참고 문서](tlab-boilerplate/docs/feature-structure.md)를 참고한다.
- `tlab-boilerplate-ui`: React UI. 화면 추가 지점은 [UI README](tlab-boilerplate-ui/README.md)를 참고한다.

API 서버 실행 명령은 `justfile`에 있다. 인증·권한 정책은 [RBAC 문서](tlab-boilerplate/docs/rbac.md)를 참고한다.
