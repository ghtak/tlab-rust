# 새 기능 구조 참고

`src/auth`는 새 기능을 설계할 때 참고할 사례다. 파일이나 계층을 그대로 복제하지 말고, 해당 기능에 실제로 필요한 책임만 둔다.

## 책임과 이름

- `route`: 경로, 요청·응답 DTO, HTTP 상태 및 오류 응답을 담당한다. DTO가 해당 엔드포인트에서만 쓰이면 route 가까이에 선언한다. 도메인 entity를 그대로 API 응답으로 노출하지 않는다.
- `usecase`: 하나의 애플리케이션 작업을 수행하고 트랜잭션의 시작·커밋 경계를 소유한다. `CreateManagedUserUsecase`, `CreateManagedUserCommand`처럼 타입에 작업 이름을 명시한다. 모듈명만으로 구분되는 `Usecase`, `Command` 같은 이름은 피한다.
- `service`: 여러 호출부에서 재사용할 작업이 실제로 생길 때 둔다. 트랜잭션 소유권을 서비스로 옮기거나, 사용처 없는 trait·서비스를 형식상 만들지 않는다. `auth/service/rbac_service.rs`는 권한 검사에 필요한 DB 조회를 조합한 사례이며, 새 기능마다 service를 만들 필요는 없다.
- `repository`: 저장·조회·삭제와 DB 오류 변환을 담당한다. SQLx 쿼리 결과는 repository 안에서 entity로 변환한다. 조회한 데이터의 유효성 등 애플리케이션 규칙은 호출하는 유스케이스에서 판단한다. 호출부에서는 `repository::user_repository::...`처럼 대상 저장소가 드러나는 국소 접근 이름을 사용한다.
- `entity`: 기능의 도메인 타입과 상태를 둔다. 값의 종류가 알려져 있고 코드에서 구분해야 한다면 문자열 대신 enum을 우선 검토한다.

## 새 백엔드 기능 연결 지점

새 기능에 필요한 파일만 `src/<기능명>` 아래에 둔다. `auth`의 하위 폴더 구성을 그대로 복제할 필요는 없다.

1. 영속 데이터가 있으면 기능 폴더의 `migrations`에 SQL을 두고 `src/migration.rs`의 목록과 `migrate` 함수에 등록한다. 현재 마이그레이션은 등록된 SQL을 실행할 때마다 다시 실행하므로 재실행 가능한 SQL로 작성한다. 새 파일을 추가하는 것만으로는 적용되지 않는다.
2. HTTP API가 있으면 기능의 `route`에 라우터를 만들고 `src/main.rs`의 앱 라우터에 연결한다. 요청·응답은 `src/api_response.rs`의 `ApiResponse`/`ApiResult` 형식을 참고한다.
3. DB나 설정 등 앱 공통 자원이 필요하면 `src/app_container.rs`의 `AppContainer`를 사용한다. 새 의존성이 실제로 필요할 때만 필드를 추가한다.
4. 인증된 요청은 `auth/access_claims.rs`의 `AccessClaims`를 사용한다. 권한 검사가 필요하면 `auth/permission.rs`의 `require`와 `auth/service/rbac_service.rs`의 흐름을 참고하고, 새 권한 코드와 부여 정책은 해당 기능의 요구 사항에 맞춰 정한다.
5. DB 통합 테스트가 필요하면 `src/test_db.rs`와 `src/test_app.rs`를 참고한다. 변경한 동작을 보장하는 테스트만 추가한다.

UI가 필요한 기능은 [`tlab-boilerplate-ui/README.md`](../../tlab-boilerplate-ui/README.md)의 연결 지점을 함께 확인한다.

## repository 인터페이스 명명 규칙

repository 함수는 조회 건수와 저장 대상을 이름에 드러낸다. 조회 조건의 기준이 되는 대상을 따라 함수를 배치하고, 반환 타입만을 기준으로 다른 repository로 옮기지 않는다.

| 이름 | 의미 |
| --- | --- |
| `search` | `SearchCriteria`를 받아 동적 조건으로 조회한다. |
| `find_by_...` | 조건에 맞는 0~1건을 조회하고, 없으면 `Ok(None)`을 반환한다. |
| `find_all_by_...` | 조건에 맞는 여러 건을 조회하고, 없으면 빈 목록을 반환한다. |
| `get_by_...` | 반드시 있어야 하는 1건을 조회하고, 없으면 `Err(NotFound)`를 반환한다. |
| `save` | repository의 주 대상 entity를 upsert한다(insert 또는 update). |
| `save_...` | 개별 entity를 upsert한다(insert 또는 update). |
| `delete_by_...` | 조건에 맞는 단건을 삭제하고 삭제 여부를 `Result<bool>`로 반환한다. |
| `delete_all_by_...` | 조건에 맞는 다건을 삭제하고 삭제 건수를 `Result<i64>`로 반환한다. |

개별 entity 삭제는 `delete_user_account_by_id`처럼 대상과 조건을 함께 이름에 드러낸다. 관계 데이터의 조회·변경 함수는 어느 대상의 연결을 관리하는지 확인하고, 기존 함수의 배치 기준에 맞춘다. 이 규칙은 새로 설계하거나 수정하는 인터페이스에 적용하며, 기존 함수를 일괄 변경할 때는 호출부와 테스트를 함께 확인한다.

## 현재 코드에서 읽을 곳

- `src/auth/route.rs`: 요청·응답 DTO와 유스케이스 호출, HTTP 오류 매핑. 현재 `/me`는 `access:manage` 권한을 검사한 뒤 JWT claims를 반환한다.
- `src/auth/usecase/create_managed_user.rs`: 명시적 타입 이름과 트랜잭션 안에서 여러 저장 작업 조합
- `src/auth/repository/user_repository.rs`: 사용자 데이터와 연결된 역할 조회·변경, SQLx 결과 → entity 변환
- `src/auth/repository/refresh_token_repository.rs`: 로그인 세션별 refresh token 해시 저장·조회·삭제
- `src/auth/repository/role_repository.rs`: 역할과 연결된 퍼미션 조회·변경
- `src/auth/access_claims.rs`, `src/auth/permission.rs`: JWT 추출과 HTTP 권한 검사. 다른 기능에서도 인증이 필요할 때 재사용한다.
- `src/auth/entity.rs`: `UserAccount`, `UserIdentity`, `UserCredential` 및 상태·제공자 타입

DB 통합 테스트는 마이그레이션이 완료된 전용 테스트 DB를 사용한다. `src/test_db.rs`에서 연결을 공통으로 관리하며, 하나의 트랜잭션에서 끝나는 테스트는 롤백하고 usecase처럼 자체 DB 연결을 여는 테스트는 생성한 데이터를 종료 시 삭제한다.

기능에 공통 작업이나 DB 접근이 없다면 대응 파일을 만들지 않는다. 반대로 파일이 커지거나 서로 다른 책임이 드러나면 그때 분리한다. 새로운 기능의 API 정책·도메인 규칙은 `auth`의 현재 구현에서 추정하지 말고 해당 요구사항으로 결정한다.
