---
name: develop-boilerplate-feature
description: tlab-boilerplate에 새 기능을 추가할 때 auth 모듈을 참고해 모듈 경계, 유스케이스, 서비스, 저장소, HTTP 입출력을 구성한다.
---

# tlab-boilerplate 기능 개발

새 기능을 만들 때 `tlab-boilerplate/src/auth`를 **참고 사례**로 사용한다. 파일이나 계층을 그대로 복제하지 말고, 해당 기능에 실제로 필요한 책임만 둔다. 작업 범위와 변경 방향은 루트 `AGENTS.md`의 작업 흐름에 따른다.

## 책임과 이름

- `route`: 경로, 요청·응답 DTO, HTTP 상태 및 오류 응답을 담당한다. DTO가 해당 엔드포인트에서만 쓰이면 route 가까이에 선언한다. 도메인 entity를 그대로 API 응답으로 노출하지 않는다.
- `usecase`: 하나의 애플리케이션 작업을 수행하고 트랜잭션의 시작·커밋 경계를 소유한다. `CreateManagedUserUsecase`, `CreateManagedUserCommand`처럼 타입에 작업 이름을 명시한다. 모듈명만으로 구분되는 `Usecase`, `Command` 같은 이름은 피한다.
- `service`: 여러 유스케이스가 트랜잭션 안에서 재사용하는 작업이 실제로 생길 때 둔다. 트랜잭션 소유권을 서비스로 옮기거나, 사용처 없는 trait·서비스를 형식상 만들지 않는다. 현재 `auth/service.rs`는 이 역할을 위한 자리일 뿐 구현 예시는 아니다.
- `repository`: 저장·조회와 DB 오류 변환을 담당한다. 저장 형식인 `row`는 repository 내부에 두고 entity로 변환한다. 호출부에서는 `repository::user_repository::...`처럼 대상 저장소가 드러나는 국소 접근 이름을 사용한다.
- `entity`: 기능의 도메인 타입과 상태를 둔다. 값의 종류가 알려져 있고 코드에서 구분해야 한다면 문자열 대신 enum을 우선 검토한다.

## 현재 코드에서 읽을 곳

- `auth/route.rs`: 요청·응답 DTO와 유스케이스 호출, HTTP 오류 매핑
- `auth/usecase/create_managed_user.rs`: 명시적 타입 이름과 트랜잭션 안에서 여러 저장 작업 조합
- `auth/repository.rs`: `postgres_user_repository` 구현을 `user_repository`라는 호출 이름으로 공개. 이 별칭은 접근 경로를 구분하기 위한 것이며 DB 교체 추상화는 아니다.
- `auth/repository/postgres_user_repository.rs`, `auth/repository/row.rs`: SQLx 쿼리와 Row → entity 변환
- `auth/entity.rs`: `UserAccount`, `UserIdentity`, `UserCredential` 및 상태·제공자 타입

기능에 공통 작업이나 DB 접근이 없다면 대응 파일을 만들지 않는다. 반대로 파일이 커지거나 서로 다른 책임이 드러나면 그때 분리한다. 새로운 기능의 API 정책·도메인 규칙은 `auth`의 현재 구현에서 추정하지 말고 해당 요구사항으로 결정한다.
