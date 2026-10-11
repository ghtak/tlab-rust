# Access token 무효화 확장안

현재 적용한 프로세스 메모리 검증과 향후 Redis 교체안을 기록한다. Redis 조회는 아직 적용되지 않았다.

## 현재 흐름

- `tlab/src/jwt.rs`의 `JwtCodec::issue()`는 `JwtClaims`에 UUID `jti`를 넣어 JWT를 서명한다. 반환하는 `SignedJwt`에는 토큰 문자열과 서명에 사용한 claims가 있다.
- `JwtCodec::verify()`는 검증된 `JwtClaims`를 반환한다. `tlab-boilerplate/src/auth/access_claims.rs`의 `AccessClaims`는 이 claims를 `jwt` 필드에 보관한다.
- 쿠키와 Bearer 인증은 `TokenService::verify_access()`에서 JWT와 현재 등록된 `jti`를 확인한다. Refresh token은 별도 DB 세션과 토큰 해시로 확인한다.

## 현재 메모리 검증

`TokenService`는 프로세스 메모리에 `user_account_id → jti`를 보관한다. 로그인과 refresh token 교체는 DB 트랜잭션 커밋 후 발급한 access token의 `jti`를 등록한다. Refresh token으로 access token만 재발급하는 경로도 새 `jti`를 등록한다. 등록된 값과 다른 access token은 거부하고, 쿠키에 유효한 refresh token이 있으면 새 access token을 발급한다. 로그아웃은 해당 access token의 `jti`가 현재 값과 일치할 때만 값을 제거한다.

이 상태는 서버를 재시작하면 사라지고 여러 서버 인스턴스 간 공유되지 않는다. 재시작 후 기존 Bearer access token은 거부되며, 유효한 refresh 쿠키가 있으면 재발급된다. 이 제약을 없애려면 아래 Redis 저장소로 교체해야 한다.

## 발급 결과 타입

발급 직후 서명에 사용한 claims를 호출자가 활용할 수 있도록 `SignedJwt`를 사용한다.

```rust
pub struct SignedJwt {
    pub token: String,
    pub claims: JwtClaims,
}
```

`SignedJwt`는 서명된 JWT 문자열과 그 문자열을 만들 때 사용한 claims의 묶음을 뜻한다. 만료 시각은 `claims.exp`로 읽는다. `SignedJwtPair`의 `access`와 `refresh`도 이 타입을 사용한다. `JwtCodec::issue()`는 서명에 사용한 바로 그 `JwtClaims`를 반환하므로, 토큰을 다시 디코딩해 발급 정보를 얻지 않는다.

## 향후 Redis로 교체

프로세스 메모리의 `user_id → jti` 저장소를 Redis로 교체한다. 로그인, refresh token 교체, refresh token을 이용한 access token 재발급 경로에서 발급한 access token의 `claims.jti`를 등록한다. 인증 시에는 JWT 서명·만료·용도를 검증한 뒤 사용자 ID로 Redis 값을 조회하고, 검증된 `claims.jti`와 일치할 때만 허용한다. 불일치하거나 값이 없으면 거부한다.

Redis 값에는 해당 access token의 남은 유효기간에 맞춘 만료 시간을 둔다. 등록에 실패한 토큰을 성공 응답으로 내보내지 않도록 발급·응답 경로의 실패 처리를 정한다. 쿠키 인증과 Bearer 인증 모두 동일한 비교를 거치게 한다. 기존 `JwtClaims.jti`는 이전 토큰과의 호환을 위해 `Option<String>`이므로, 이 검사에서는 `None`을 거부하고 기존 토큰의 전환 방식을 적용 전에 결정한다.

`iat`는 현재 초 단위이며 같은 초에 발급한 토큰을 구별하지 못한다. 개별 토큰을 식별하는 이 요구에는 이미 발급 중인 UUID `jti`가 적합하다. 이 설계는 새 access token 발급 시 같은 사용자의 이전 access token을 모두 무효화한다. 여러 기기나 세션을 동시에 허용하려면 Redis 키에 기존 `AppClaims.session_id`를 포함해 세션별로 비교하는 설계를 별도로 선택해야 한다.

## 적용 전 확인할 동작

- 로그인과 두 refresh 경로에서 발급한 access token이 Redis에 등록되는지 확인한다.
- 같은 사용자의 새 access token 발급 후 이전 토큰은 거부되고 새 토큰은 허용되는지 확인한다.
- 쿠키와 Bearer 인증에 동일한 규칙이 적용되는지 확인한다.
- Redis 값이 없거나 등록·조회에 실패할 때 인증·응답 처리가 의도한 대로 동작하는지 확인한다.
