# JWT 구현안

## 목표와 배치

- `tlab/src/jwt.rs`에 Ed25519 키 파일 생성과 JWT 발급·검증을 구현한다. `tlab/src/lib.rs`에서 `jwt` 모듈을 공개한다.
- 한 번의 발급 호출에서 access token과 refresh token을 함께 반환한다.
- 이 문서는 JWT 자체의 기능만 다룬다. HTTP 응답, 사용자 조회, 재발급 및 로그아웃 흐름은 포함하지 않는다.

JWT에는 TLS 인증서가 필요하지 않다. `TlsCertificateFiles`와 비슷한 파일 관리 타입은 인증서가 아닌 키 쌍을 나타내도록 `EdDsaKeyFiles`로 명명한다.

## `tlab::jwt`의 역할

### 키 파일

```rust
pub struct EdDsaKeyFiles {
    pub private_key: String,
    pub public_key: String,
}
```

- `generate()`는 `rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519)`로 키 쌍을 만들고, 개인키는 PKCS#8 PEM, 공개키는 PUBLIC KEY PEM으로 저장한다. 기존 `tlab`의 `rcgen` 의존성을 재사용한다.
- 생성 전 두 경로가 비어 있거나 동일한지 확인한다. 어느 한 파일이라도 이미 있으면 덮어쓰지 않고 오류를 반환한다.
- 시작 시에는 **두 파일 모두 있음 → 로드**, **둘 다 없음 → 설정에서 허용한 경우에만 생성**, **한 파일만 있음 → 오류**로 처리한다. 개인키 파일의 접근 권한을 제한하고 저장소에 커밋하지 않는다.
- 앱 재시작마다 키를 다시 만들지 않는다. 키가 바뀌면 이전 JWT의 서명을 검증할 수 없다.

### 발급과 검증

`JwtCodec`는 시작 시 키와 검증 설정을 한 번 읽어 보관한다. `jsonwebtoken`의 EdDSA 및 PEM 지원을 사용하며, 암호 연산을 직접 구현하지 않는다.

```rust
pub struct JwtConfig {
    pub key_files: EdDsaKeyFiles,
    pub issuer: String,
    pub audience: String,
    pub access_token_ttl_seconds: u64,
    pub refresh_token_ttl_seconds: u64,
    pub generate_if_missing: bool,
}

pub struct IssuedToken {
    pub token: String,
    pub expires_in: u64,
}

pub struct TokenPair {
    pub access: IssuedToken,
    pub refresh: IssuedToken,
}

pub enum TokenUse {
    Access,
    Refresh,
}

pub struct JwtClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub iat: u64,
    pub exp: u64,
    pub token_use: TokenUse,
}

impl JwtCodec {
    pub fn new(config: &JwtConfig) -> tlab::Result<Self>;
    pub fn issue_pair(&self, subject: &str) -> tlab::Result<TokenPair>;
    pub fn verify(&self, token: &str) -> tlab::Result<JwtClaims>;
}
```

- `issue_pair`는 서로 다른 만료 시각을 가진 두 JWT를 발급한다. 각 토큰에는 `sub`, `iss`, `aud`, `iat`, `exp`와 용도를 나타내는 `token_use`(`access` 또는 `refresh`)를 담는다. `TokenPair`는 두 토큰 문자열과 각각의 만료까지 남은 초를 반환한다.
- `tlab`은 사용자 ID 타입을 가정하지 않고 `sub`를 문자열로 다룬다. 두 토큰은 같은 Ed25519 키 쌍으로 서명한다.
- `verify`는 허용 알고리즘을 `EdDSA`로 고정하고 서명, 만료, 발급자, 대상, `sub`의 존재 여부를 확인한 뒤 `JwtClaims`를 반환한다. 호출자는 토큰을 사용하기 전에 `claims.token_use`가 해당 작업의 용도와 맞는지 확인한다.
- 잘못되었거나 만료된 토큰은 `tlab::Error::InvalidToken`으로, 키 로딩·설정·발급 오류는 내부 오류로 구분한다. 원본 토큰과 개인키는 로그에 남기지 않는다.
- 두 TTL은 양수여야 하며 발급 시 시각 계산의 범위를 검사한다.

## 설정 예시

개발 환경에서는 `generate_if_missing: true`, 운영 환경에서는 사전 배포한 키를 사용하도록 `false`로 둔다.

```yaml
jwt:
  key_files:
    private_key: "jwt-private.pem"
    public_key: "jwt-public.pem"
  issuer: "tlab-boilerplate"
  audience: "tlab-boilerplate-api"
  access_token_ttl_seconds: 900
  refresh_token_ttl_seconds: 604800
  generate_if_missing: true
```

## 검증 범위

- 키 파일 생성 후 재로드, 기존 파일 덮어쓰기 거부, 한 파일만 있는 상태에서 오류 반환
- access·refresh 동시 발급과 각각의 검증 왕복
- 만료·변조·다른 공개키·다른 발급자·다른 대상 거부
- 검증 결과에 발급 시 지정한 `token_use`가 보존되는지 확인

Refresh token의 보관·교체·폐기, 즉시 로그아웃, JWKS와 키 교체는 이 모듈의 범위에 포함하지 않는다. 서명과 클레임 검증만으로 refresh token의 재사용 여부를 판단할 수는 없다.

## 참고

- [JWT 등록 클레임](https://www.rfc-editor.org/rfc/rfc7519.html)
- [JWT 보안 권고](https://www.rfc-editor.org/rfc/rfc8725.html)
- [JOSE의 Ed25519 키 형식](https://www.rfc-editor.org/rfc/rfc8037.html)
- [jsonwebtoken의 EdDSA 지원](https://docs.rs/jsonwebtoken/latest/jsonwebtoken/enum.Algorithm.html)
