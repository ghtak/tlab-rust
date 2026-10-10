# RBAC 인증 연계 구현안

## 목표와 현재 상태

[RBAC 구조](../rbac.md)의 역할·퍼미션 테이블을 HTTP 권한 검사에 연결한다. access token에는 사용자의 역할 ID만 담고, 서버가 역할별 퍼미션을 조회해 `리소스:액션` 코드를 검사한다. 파일 소유자·팀·그룹에 따른 대상별 접근 판단은 이 작업에 포함하지 않는다.

현재 JWT에는 사용자 ID인 `sub`만 있으며, `AccessClaims`는 선택적 Bearer 토큰 추출에 사용된다. `POST /api/v1/auth/user`는 라우트에서 비활성화되어 있고, 아직 RBAC를 검사하는 요청은 없다.

## 토큰과 요청의 데이터 형태

공용 `tlab::jwt::JwtClaims`에는 앱별 타입 대신 선택적인 `app: Option<serde_json::Value>`를 추가한다. 기존 토큰을 역직렬화할 수 있도록 필드에 `serde(default)`를 적용한다. `tlab`은 이 값의 내용이나 역할의 의미를 해석하지 않는다.

보일러플레이트의 `AccessClaims`가 검증된 JWT와 역할 ID를 보관한다.

```rust
struct AccessClaims {
    jwt: tlab::jwt::JwtClaims,
    app: AppClaims,
}

struct AppClaims {
    role_ids: Vec<i64>,
    session_id: Uuid,
}
```

- 비밀번호 확인 후 DB에서 현재 역할 ID를 조회하고 새 `session_id`를 생성한다. `issue_pair(subject, app)`으로 access·refresh token에 같은 `app: { "role_ids": [...], "session_id": "..." }`를 넣고, DB에는 세션별 refresh token 해시를 저장한다. refresh 기능을 구현할 때 역할 ID는 토큰에서 재사용하지 않고 DB에서 다시 조회한다.
- `AccessClaims` 추출기는 서명·만료·발급자·대상·`TokenUse::Access`를 검증한 뒤 `app`을 `AppClaims`로 파싱한다. 계정 ID가 필요한 경로는 `user_account_id()`로 `sub`를 파싱하며, 정수가 아니면 401로 거부한다. `app`에 필수 필드가 없어도 401로 거부한다. 역할 ID는 신뢰할 수 있는 DB 조회 결과로만 발급한다.
- 현재 `Option<AccessClaims>` 추출은 `/me`에 유지한다. 보호된 경로에는 Bearer 토큰을 필수로 요구하는 추출을 제공하고, 두 추출 방식은 같은 검증 함수를 사용한다. 토큰 없음·무효는 401, 유효한 토큰에 필요한 퍼미션이 없음은 403으로 구분한다.

## 구현 순서

1. `auth/repository`에 사용자 ID로 역할 ID를 조회하고, 역할 ID 목록으로 중복 없는 퍼미션 코드를 조회하는 함수를 추가한다. 역할이 없는 사용자는 빈 목록을 반환한다.
2. `RbacService`에 역할 ID 목록과 퍼미션 코드를 받아 허용 여부를 판단하는 기능을 둔다. 역할에 연결된 퍼미션만 확인하며, 알 수 없는 역할·퍼미션은 허용하지 않는다. 먼저 DB 조회로 동작을 검증한다.
3. `JwtClaims.app` 발급·검증을 추가하고, 로그인 시 역할을 읽어 access token을 발급한다. refresh 기능을 구현할 때는 refresh token의 기존 역할을 재사용하지 않고 DB에서 다시 조회한다.
4. `AccessClaims`를 위 구조로 확장하고 필수·선택적 추출을 연결한다. `RbacService`는 JWT 파싱을 담당하지 않고, `AccessClaims`는 퍼미션 판정을 담당하지 않는다.
5. 로그인에서는 활성 계정에만 토큰을 발급한다. `POST /api/v1/auth/user`를 다시 제공할 때 `access:manage`를 검사한다. 라우트 활성화 전에 기본 관리자 계정의 고정 비밀번호 `passwd`를 제거하거나 변경한다. 이미 발급된 토큰에 대한 계정 정지의 즉시 반영은 별도 단계로 둔다.

역할별 퍼미션 조회가 병목으로 확인되면 `RbacService`에 `role_id → permission code 집합` 캐시를 추가한다. 역할의 퍼미션을 변경하는 경로에서는 해당 캐시를 무효화해야 하며, 서버가 여러 대라면 각 서버에 변경을 전파해야 한다. 사용자별 역할은 JWT에 들어 있으므로 역할 부여·회수는 기존 access token 만료 전까지 반영되지 않는다. 즉시 회수가 필요해지면 사용자별 버전 확인이나 토큰 무효화를 별도 단계로 설계한다.

## 검증 범위

- 역할이 없거나 필요한 퍼미션이 없는 사용자는 보호된 작업을 수행할 수 없고, `admin`의 `access:manage`는 허용된다.
- 토큰 없음, 잘못된 서명·토큰 종류·`app` 형식은 거부한다. `app`이 없는 기존 access token은 인증되더라도 RBAC 퍼미션이 없다.
- access·refresh token에는 같은 세션 ID가 들어가고, 로그아웃은 해당 사용자의 해당 세션 refresh token만 삭제한다. 이미 발급된 access token은 만료 전까지 유효하다.
