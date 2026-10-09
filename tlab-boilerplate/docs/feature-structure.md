# 새 기능 구조 참고

`src/auth`는 새 기능을 설계할 때 참고할 사례다. 파일이나 계층을 그대로 복제하지 말고, 해당 기능에 실제로 필요한 책임만 둔다.

## 책임과 이름

- `route`: 경로, 요청·응답 DTO, HTTP 상태 및 오류 응답을 담당한다. DTO가 해당 엔드포인트에서만 쓰이면 route 가까이에 선언한다. 도메인 entity를 그대로 API 응답으로 노출하지 않는다.
- `usecase`: 하나의 애플리케이션 작업을 수행하고 트랜잭션의 시작·커밋 경계를 소유한다. `CreateManagedUserUsecase`, `CreateManagedUserCommand`처럼 타입에 작업 이름을 명시한다. 모듈명만으로 구분되는 `Usecase`, `Command` 같은 이름은 피한다.
- `service`: 여러 호출부에서 재사용할 작업이 실제로 생길 때 둔다. 트랜잭션 소유권을 서비스로 옮기거나, 사용처 없는 trait·서비스를 형식상 만들지 않는다. `auth/service/rbac_service.rs`는 권한 검사에 필요한 DB 조회를 조합한 사례이며, 새 기능마다 service를 만들 필요는 없다.
- `repository`: 저장·조회·삭제와 DB 오류 변환을 담당한다. SQLx 쿼리 결과는 repository 안에서 entity로 변환한다. 조회한 데이터의 유효성 등 애플리케이션 규칙은 호출하는 유스케이스에서 판단한다. 호출부에서는 `repository::user_repository::...`처럼 대상 저장소가 드러나는 국소 접근 이름을 사용한다.
- `entity`: 기능의 도메인 타입과 상태를 둔다. 값의 종류가 알려져 있고 코드에서 구분해야 한다면 문자열 대신 enum을 우선 검토한다.

## 현재 코드에서 읽을 곳

- `src/auth/route.rs`: 요청·응답 DTO와 유스케이스 호출, HTTP 오류 매핑. 현재 `/me`는 `user:manage` 권한을 검사한 뒤 JWT claims를 반환한다.
- `src/auth/usecase/create_managed_user.rs`: 명시적 타입 이름과 트랜잭션 안에서 여러 저장 작업 조합
- `src/auth/repository/user_repository.rs`: 사용자 데이터와 연결된 역할 조회·변경, SQLx 결과 → entity 변환
- `src/auth/repository/refresh_token_repository.rs`: 로그인 세션별 refresh token 해시 저장·조회·삭제
- `src/auth/repository/role_repository.rs`: 역할과 연결된 퍼미션 조회·변경
- `src/auth/access_claims.rs`, `src/auth/permission.rs`: JWT 추출과 HTTP 권한 검사. 다른 기능에서도 인증이 필요할 때 재사용한다.
- `src/auth/entity.rs`: `UserAccount`, `UserIdentity`, `UserCredential` 및 상태·제공자 타입

DB 통합 테스트는 마이그레이션이 완료된 전용 테스트 DB를 사용한다. `src/test_db.rs`에서 연결을 공통으로 관리하며, 하나의 트랜잭션에서 끝나는 테스트는 롤백하고 usecase처럼 자체 DB 연결을 여는 테스트는 생성한 데이터를 종료 시 삭제한다.

기능에 공통 작업이나 DB 접근이 없다면 대응 파일을 만들지 않는다. 반대로 파일이 커지거나 서로 다른 책임이 드러나면 그때 분리한다. 새로운 기능의 API 정책·도메인 규칙은 `auth`의 현재 구현에서 추정하지 말고 해당 요구사항으로 결정한다.
