# 시스템 대시보드와 메트릭 구현안

## 목적과 현재 구조

`/` 대시보드에 호스트 CPU·메모리, 메모리 사용량 상위 프로세스와 애플리케이션의 처리 중 HTTP 요청 수, PostgreSQL 풀 상태를 표시한다. CPU와 메모리의 최근 변화는 그래프로 표시한다. 구현 시 결정한 범위와 API 계약을 이 문서에 기록한다.

UI의 `src/features/dashboard/pages/DashboardPage.tsx`는 실제 스냅샷을 표시한다. `src/routes/_app.index.tsx`는 로그인만 확인하는 `_app` 아래의 `/`에 이 화면을 연결한다. 백엔드는 Axum 라우터를 `src/main.rs`에서 조합하고 `AppContainer`가 SQLx 데이터베이스 풀과 메트릭 상태를 보유한다.

## 범위와 의미

| 화면 항목 | 수집값과 표시 기준 |
| --- | --- |
| CPU | 전체 및 코어별 사용률. 논리 코어 수는 코어별 값의 개수에서 구한다. 사용률은 0~100%로 표시한다. |
| 메모리 | 전체·사용 가능 메모리(bytes). 사용량은 `전체 − 사용 가능`, 사용률은 `사용량 / 전체`에서 파생한다. |
| 프로세스 | 서버가 접근 가능한 프로세스 중 메모리 사용량 상위 20개. PID, 이름, 상주 메모리(bytes), CPU 사용률을 표시한다. |
| 처리 중 HTTP 요청 | 현재 미들웨어 안에서 처리 중인 요청 수. 열린 TCP 소켓 수나 로그인 세션 수가 아니다. |
| PostgreSQL 풀 | 최대 연결 수, 현재 열린 연결 수, 유휴 연결 수. 사용 중 수는 `열린 수 − 유휴 수`로 파생한다. |

CPU·메모리의 기준은 **호스트 OS 전체**다. 컨테이너의 cgroup 메모리·CPU 제한은 반영하지 않으며 화면과 API의 `scope: "host"`로 표시한다. 프로세스 목록은 현재 프로세스 네임스페이스에서 서버가 볼 수 있는 범위이므로, 컨테이너 안에서는 호스트 전체 프로세스 목록이 아닐 수 있다.

## 경계와 데이터 흐름

```text
sysinfo ── 5초 수집 작업 ── 최신 시스템 스냅샷 ─┐
                                                   ├─ GET /api/v1/dashboard/snapshot ─ UI
HTTP 요청 미들웨어 ── 현재 처리 중 요청 수 ────────┤
tlab::sqlxdb::Database ── SQLx 풀 상태 ───────────┘
```

- `tlab-boilerplate/src/metrics.rs`는 수집 주기, `sysinfo` 상태, 스냅샷 저장, 처리 중 요청 게이지를 소유한다. 현재 사용처가 하나이므로 범용 레지스트리·trait·repository는 만들지 않는다.
- `tlab/src/sqlxdb.rs`는 공용 풀 래퍼의 읽기 전용 상태 조회 메서드만 제공한다. 데이터베이스 설정이나 풀의 내부 객체를 API 계층에 노출하지 않는다.
- `tlab-boilerplate/src/dashboard/route.rs`는 인증, HTTP 상태와 응답 DTO를 담당한다. 사용량 등 원본에서 계산 가능한 값은 응답에 중복으로 넣지 않는다.
- `AppContainer`는 최신 스냅샷과 요청 게이지에 접근할 수 있는 메트릭 상태를 보유한다. 수집 작업의 시작·종료는 `main.rs`에서 서버 실행 수명에 맞춘다.

## 수집 작업

1. 서버 시작 시 `sysinfo::System` 하나를 생성하고, CPU의 첫 기준 측정을 수행한다. 첫 CPU 사용률은 이전 측정 구간이 없어 유효한 값으로 표시하지 않는다.
2. `tokio::spawn`한 작업 하나가 `tokio::time::interval`을 5초 주기로 실행한다. 밀린 틱을 연속 실행하지 않도록 `MissedTickBehavior::Skip`을 설정한다.
3. 각 틱에서 `spawn_blocking`으로 `sysinfo`의 동기 갱신과 스냅샷 생성을 수행한다. 비동기 런타임 스레드에서 전체 프로세스 목록을 직접 갱신하지 않는다. `System`은 작업 간 재사용하고 동시에 갱신하지 않는다.
4. CPU·메모리는 매 틱, 프로세스 정보는 세 번째 틱마다 갱신한다. 프로세스 갱신에는 필요한 메모리·CPU·이름 정보만 요청하고, 불필요한 task/thread 세부 정보는 제외한다. 종료된 프로세스는 다음 프로세스 갱신에서 제거한다.
5. 갱신 결과를 `sampled_at` 및 `processes_sampled_at`과 함께 저장한다. API는 마지막 완성본을 읽기만 하며, 요청마다 `sysinfo`를 실행하거나 수집 작업 종료를 기다리지 않는다. 처음 유효한 스냅샷이 만들어지기 전에는 `503`과 일관된 API 오류를 반환한다.
6. `spawn_blocking` 실패는 기록하고 마지막 성공 스냅샷을 유지한다. UI는 측정 시각으로 오래된 값을 구분한다. 서버 종료 시 주기 작업을 중단한다.

`sysinfo`의 CPU 사용률은 두 측정값 사이의 차이로 계산된다. 수집 간격은 라이브러리의 최소 CPU 갱신 간격보다 길게 둔다. 프로세스 갱신 비용과 Linux의 task 수집 여부는 실제 배포 환경에서 확인한다. [sysinfo 문서](https://docs.rs/sysinfo/latest/sysinfo/struct.System.html)

## 처리 중 HTTP 요청 계측

앱 라우터의 바깥쪽에 계측 미들웨어를 한 번 적용한다. 요청이 계층에 들어오면 `AtomicU64` 게이지를 증가시키고 종료 시 감소시킨다. 증가 뒤에 감소를 보장하는 수명 가드를 사용해 요청 취소나 오류로 future가 중단되어도 값이 남지 않게 한다. 인증 미들웨어에서 일찍 응답하는 요청과 정적 파일 요청도 같은 기준으로 센다. 응답을 만드는 `/snapshot` 요청 자신 하나는 응답 값에서 빼므로, 다른 요청이 없으면 화면에는 0이 표시된다.

이 값은 HTTP 요청의 처리 중 수이며 keep-alive TCP 연결 수가 아니다. 실제 소켓 연결 수가 필요해지면 HTTP/HTTPS 서버 양쪽의 연결 수명에 계측을 추가하는 별도 작업으로 다룬다. 누적 요청 수나 상태별 지연 시간은 이번 화면의 필수 지표가 아니다.

## PostgreSQL 풀 상태

`Database<DB>`에 `PoolStatus { max_connections, size, idle }` 같은 읽기 전용 결과를 추가한다. `max_connections`는 풀 옵션, `size`와 `idle`은 SQLx 풀의 조회 메서드에서 얻는다. 사용 중 수는 API 소비자가 계산한다. 샘플 간 값이 바뀔 수 있으므로 정확한 원자적 스냅샷이나 PostgreSQL 서버 전체 연결 수로 해석하지 않는다. 현재 앱 인스턴스의 **이 풀 하나**에 대한 값이다. [SQLx Pool 문서](https://docs.rs/sqlx/latest/sqlx/struct.Pool.html)

## API 계약과 권한

`GET /api/v1/dashboard/snapshot`을 기존 `ApiResponse<T>` 형식으로 제공한다. 예시 값은 다음과 같다.

```json
{
  "data": {
    "sampled_at": "2026-10-10T12:00:05Z",
    "scope": "host",
    "cpu": {
      "total_usage_percent": 28.5,
      "cores_usage_percent": [12.0, 36.0, 31.0, 35.0]
    },
    "memory": { "total_bytes": 17179869184, "available_bytes": 6442450944 },
    "processes_sampled_at": "2026-10-10T12:00:00Z",
    "processes": [
      { "pid": 1234, "name": "example", "memory_bytes": 536870912, "cpu_usage_percent": 5.2 }
    ],
    "application": {
      "http_requests_in_flight": 2,
      "database_pool": { "max_connections": 16, "size": 7, "idle": 4 }
    }
  }
}
```

CPU 첫 측정치가 유효하지 않으면 관련 사용률을 `null`로 응답한다. 프로세스 목록은 서버에서 메모리 내림차순으로 정렬해 상위 20개로 제한한다. 요청한 시각과 실제 측정 시각을 혼동하지 않도록 `sampled_at`을 수집 시각으로 사용한다. 브라우저 캐시를 막고, 인증 실패는 `401`, 초기 스냅샷 부재는 `503`으로 구분한다.

**권한 정책:** 대시보드와 API는 로그인한 사용자 전체에게 공개한다. 서버는 기존 `AccessClaims`로 인증한다. `metrics:read` 퍼미션이나 RBAC 마이그레이션은 추가하지 않는다. 익명 요청에는 데이터를 반환하지 않는다.

## UI 동작

- `src/features/dashboard/types.ts`, `services/api.ts`, `services/queries.ts`에 응답 타입과 TanStack Query 조회를 둔다. 기존 `src/api.ts` 클라이언트와 쿠키 인증 흐름을 사용한다.
- 화면이 보이는 동안 5초마다 조회한다. 새로고침이나 탭 재진입 후 그래프는 새로 시작하며, 브라우저 메모리에 최근 30분, 최대 360개 측정점만 보관한다. 같은 `sampled_at` 값은 중복 추가하지 않는다. 일시적 조회 실패 시 이미 받은 점은 유지하고 오류·마지막 갱신 시각을 표시한다. 이력은 서버나 DB에 저장하지 않는다.
- CPU 전체 사용률과 메모리 사용률 그래프를 별도로 배치한다. 코어별 사용률은 현재값 목록으로 표시한다. 기존 차트 의존성이 없으므로 두 시계열은 작은 SVG 그래프로 구현하고 축·단위·현재값을 텍스트로 제공한다.
- 메모리는 `total_bytes − available_bytes`, 풀의 사용 중 연결은 `size − idle`을 UI에서 파생한다. 프로세스 표는 메모리 내림차순이며 빈 목록·수집 대기·오류 상태를 구분한다.
- 기존 권한 관리 바로가기와 임시 숫자는 제거한다. 권한 관리 이동 경로는 기존 사이드메뉴에 남는다.

## Prometheus 연계 방침

1차 구현에는 `/metrics`와 Prometheus 서버를 포함하지 않는다. 화면 그래프는 브라우저가 열려 있는 동안만 이력을 보유한다. 서버 재시작 또는 새로고침 후 과거 추세가 필요하면 시계열 저장소를 도입한다.

추후 연계 시 같은 수집 스냅샷과 요청 게이지·풀 수치를 Prometheus 형식으로 노출한다. 브라우저용 JSON API와 scrape 경로는 분리하고, scrape 경로에는 브라우저 쿠키 인증 대신 네트워크 또는 별도 인증 정책을 적용한다. PID·프로세스 이름은 값 종류가 계속 늘어나는 라벨이므로 기본 scrape 지표에 넣지 않는다. 메모리는 bytes, 비율은 0~1 등 일관된 단위를 사용한다. [Prometheus 명명·라벨 지침](https://prometheus.io/docs/practices/naming/)

## 구현 순서와 필수 검증

1. `tlab-boilerplate`에 `sysinfo` 의존성을 추가하고 `metrics` 수집 작업과 최신 스냅샷을 구현한다. 첫 CPU 측정치, 프로세스 상위 20개 정렬, 실패 후 마지막 값 유지 동작을 검증한다.
2. `tlab::sqlxdb::Database`의 풀 상태 조회와 앱 HTTP 미들웨어 게이지를 연결한다. 연결 획득·반환에 따른 풀 수치와 요청 완료·취소 후 게이지 복원을 확인한다.
3. API route와 `AppContainer`를 연결한다. 인증 없음·정상 조회·초기 수집 전 응답을 검증한다. 새 RBAC 마이그레이션은 없다.
4. UI를 API에 연결하고 CPU·메모리 그래프와 프로세스 표를 만든다. 폴링 중복점, 재조회 오류, 로그인 여부별 화면 접근을 확인한다.
5. 권한 또는 화면 구성이 달라지면 `docs/_index.md`, UI 권한 안내 문서와 API 계약 문서를 실제 구현에 맞춰 갱신한다. 마지막에 관련 Rust 테스트와 UI 빌드·필요한 UI 테스트를 실행한다.

## 확정된 결정

1. 로그인한 사용자는 역할과 관계없이 대시보드와 JSON API를 조회한다.
2. CPU·메모리 수치는 호스트 기준으로 표시한다. 프로세스 목록은 서버에서 볼 수 있는 범위를 표시한다.
3. 그래프 이력은 브라우저 메모리에 최근 30분만 보관한다. 새로고침·재접속 뒤 과거 이력은 제공하지 않는다.
