# MacAgentWatch 코드베이스 종합 점검 보고서

**점검일**: 2026-02-07
**프로젝트**: agent-watch v0.3.0 (Phase 2)
**기술 스택**: Rust (core/cli) + Swift (macOS app) + UniFFI bridge
**목적**: AI 에이전트 활동 모니터링 및 보안 도구
**코드량**: ~7,200줄 (Rust 13파일, Swift 7파일)

---

## 1. 전체 요약 대시보드

| 점검 영역 | 🔴 Critical | 🟠 Major | 🟡 Minor | 🟢 Good |
|-----------|:-----------:|:--------:|:--------:|:-------:|
| 보안 | 0 | 0 | 2 | 8 |
| 로깅/모니터링 | 0 | 0 | 2 | 8 |
| 코드 품질 | 0 | 1 | 7 | 3 |
| 프로젝트 구조 | 0 | 1 | 6 | 5 |
| 아키텍처 설계 | 0 | 2 | 5 | 10 |
| 의존성 관리 | 0 | 1 | 6 | 7 |
| 메모리 관리 | ~~3~~ 0 | 3 | 2 | ~~2~~ 5 |
| 동시성/스레드 안전성 | ~~3~~ 0 | 3 | 2 | ~~0~~ 3 |
| 에러 처리 | 3 | 3 | 3 | 3 |
| 데이터 영속성 | 2 | 3 | 4 | 5 |
| 성능 최적화 | 0 | 3 | 5 | 2 |
| 접근성/국제화 | 3 | 4 | 4 | 1 |
| 테스트 커버리지 | ~~1~~ 0 | 3 | 1 | ~~1~~ 2 |
| CI/CD/빌드 | ~~2~~ 0 | 3 | 3 | ~~2~~ 4 |
| **합계** | **~~17~~ ~~14~~ 8** | **30** | **52** | **~~57~~ ~~60~~ 66** |

---

## 2. 즉시 조치 목록 (🔴 Critical)

### 2.1 인프라/빌드 — ✅ 조치 완료 (2026-02-07)

| # | 영역 | 파일/위치 | 설명 | 조치 결과 |
|---|------|-----------|------|-----------|
| ~~C1~~ | CI/CD | `.github/workflows/ci.yml` | ~~**CI 파이프라인 완전 부재**~~ | ✅ **해결**: GitHub Actions CI 워크플로우 추가 — `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo build --release` (macos-latest, rust-cache 포함) |
| ~~C2~~ | CI/CD | `Makefile` | ~~**Makefile 없음**~~ | ✅ **해결**: Makefile 추가 — `make test`, `make build`, `make lint`, `make fmt`, `make build-ffi`, `make build-app`, `make clean`, `make help` 타겟 정의 |
| ~~C3~~ | 테스트 | `app/.../MacAgentWatchTests/` | ~~**Swift 앱 테스트 0개**~~ | ✅ **해결**: XCTest 71개 테스트 케이스 추가 — `MonitoringTypesTests.swift` (33), `CoreBridgeTests.swift` (16), `MonitoringViewModelTests.swift` (22). Xcode 테스트 타겟 등록 필요 |

### 2.2 동시성/메모리 — ✅ 조치 완료 (2026-02-07)

| # | 영역 | 파일/위치 | 설명 | 조치 결과 |
|---|------|-----------|------|-----------|
| ~~C4~~ | 동시성 | `core/src/wrapper.rs` | ~~**MonitoringOrchestrator stop 순서 race**~~ | ✅ **해결**: 2단계 종료 구현 — Phase 1: 모든 subsystem에 `signal_stop()` 호출 (non-blocking), Phase 2: `stop()` + `join()` 순차 실행. 이벤트 손실 방지 |
| ~~C5~~ | 동시성 | `core/src/ffi.rs` | ~~**FfiMonitoringEngine Mutex 경쟁**~~ | ✅ **해결**: `SessionState` enum 도입 (Idle/Starting/Active/Stopping). 상태 전이 시 원자적 검증, 실패 시 이전 상태 롤백. 동시 start/stop 호출 차단 |
| ~~C6~~ | 동시성 | `core/src/fswatch.rs` | ~~**FSEvents 스레드 channel disconnection**~~ | ✅ **해결**: `catch_unwind` + `AssertUnwindSafe` 패턴으로 panic 발생 시에도 `shutdown_observe()` 호출 보장. panic 후 `resume_unwind`로 전파 |
| ~~C7~~ | 메모리 | `fswatch.rs`, `netmon.rs`, `process_tracker.rs` | ~~**stop_flag race condition**~~ | ✅ **해결**: `Arc<Mutex<bool>>` → `Arc<AtomicBool>` 전환 (3개 파일). Mutex poisoning 제거, lock-free 동작. 각 subsystem에 `signal_stop()` 메서드 추가 |
| ~~C8~~ | 메모리 | `core/src/netmon.rs` | ~~**seen_connections 전체 clear**~~ | ✅ **해결**: `SeenConnectionsCache` 세대별(generational) 캐시 구현. current/previous 2개 HashSet 사용, 용량 초과 시 이전 세대만 교체. 중복 이벤트 폭증 방지 |
| ~~C9~~ | 메모리 | `core/src/wrapper.rs` | ~~**session_logger `Arc<Mutex>` 불필요한 복잡성**~~ | ✅ **해결**: `Option<Arc<Mutex<SessionLogger>>>` → `Option<Mutex<SessionLogger>>`. 불필요한 Arc 제거, 단일 스레드 사용 근거 문서화 |

### 2.3 에러 처리

| # | 영역 | 파일/위치 | 설명 | 권장 조치 |
|---|------|-----------|------|-----------|
| C10 | 에러 | `core/src/wrapper.rs:379-381` | **SessionLogger 생성 실패를 조용히 무시** — 로깅 미작동을 사용자가 알 수 없음 | 실패 시 경고 로그 추가 또는 Result 반환 |
| C11 | 에러 | `core/src/wrapper.rs:247-249` | **FSWatcher 시작 실패 무시** — 파일시스템 모니터링 비활성화 모를 수 있음 | 에러 로깅 또는 상위 전파 |
| C12 | 에러 | `core/src/wrapper.rs:286-288` | **NetworkMonitor 시작 실패 무시** — 네트워크 모니터링 비활성화 모를 수 있음 | 에러 로깅 또는 상위 전파 |

### 2.4 데이터/영속성

| # | 영역 | 파일/위치 | 설명 | 권장 조치 |
|---|------|-----------|------|-----------|
| C13 | 영속성 | `core/src/storage.rs:106-107` | **SessionLogger header/footer write 후 flush 미보장** | write 후 즉시 `flush()` 호출 |
| C14 | 영속성 | `core/src/wrapper.rs:598-614` | **log_session_start/end 에러 무시 (`let _ =`)** — 세션 메타데이터 손실 | 실패 시 최소 `eprintln!` 추가 |

### 2.5 접근성/국제화

| # | 영역 | 파일/위치 | 설명 | 권장 조치 |
|---|------|-----------|------|-----------|
| C15 | i18n | Swift UI 전체 | **모든 UI 문자열 하드코딩** — i18n 불가 | `Localizable.strings` + `LocalizedStringKey` 도입 |
| C16 | 접근성 | Swift Views 전체 | **접근성 레이블 전무** — VoiceOver 사용 불가 | `.accessibilityLabel()` 추가 |
| C17 | 접근성 | `core/src/event.rs:24-32` | **이모지 하드코딩** — 터미널 환경에 따라 미표시 | 텍스트 대체 옵션 제공 |

---

## 3. 단기 개선 로드맵 (🟠 Major) — 1~2주 계획

### Week 1: 빌드/안정성

| # | 영역 | 설명 | 우선순위 |
|---|------|------|---------|
| M1 | 빌드 | `edition = "2024"` → `"2021"` 변경 (불안정 에디션) | P0 |
| M2 | 코드 품질 | `CoreBridge.swift` FFI 실제 연결 (TODO 8개 해소) | P0 |
| M3 | 버전 | `Cargo.toml`(0.2.0) ↔ Swift(0.3.0) 버전 통일 | P1 |
| M4 | 동시성 | stdin forwarding thread join 보장 (`wrapper.rs:451`) | P1 |
| M5 | 동시성 | PTY output thread와 main thread 동기화 (`wrapper.rs:477`) | P1 |
| M6 | 동시성 | ProcessTracker HashMap lock 시간 최적화 | P1 |
| M7 | 메모리 | unsafe 블록 안전성 검증 강화 (`netmon.rs:289-388`) | P1 |
| M8 | 메모리 | 프로세스 트리 BFS max_depth 기본값 설정 | P2 |

### Week 2: 테스트/성능

| # | 영역 | 설명 | 우선순위 |
|---|------|------|---------|
| M9 | 테스트 | FSWatch/NetMon macOS 통합 테스트 추가 | P1 |
| M10 | 테스트 | MonitoringOrchestrator 통합 테스트 추가 | P1 |
| M11 | 성능 | `process_tracker.rs:269` get_descendants O(n²) 최적화 | P1 |
| M12 | 성능 | `netmon.rs` 폴링 간격 조정 (500ms → 1s+) | P2 |
| M13 | 성능 | `wrapper.rs` line_buffer `String` → `VecDeque` 최적화 | P2 |
| M14 | 영속성 | Drop에서 flush 실패 로깅 (`storage.rs:144`) | P1 |
| M15 | 영속성 | write_event 주기적 auto-flush 추가 | P2 |
| M16 | i18n | CLI 메시지 i18n 라이브러리(fluent-rs) 도입 | P2 |
| M17 | 접근성 | 색상 외 정보 전달 수단 추가 (아이콘+텍스트) | P2 |
| M18 | 빌드 | `build-ffi.sh` 필수 도구 존재 여부 검증 추가 | P2 |

---

## 4. 장기 개선 제안 (🟡 Minor 및 아키텍처 방향)

### 4.1 아키텍처

- MonitoringOrchestrator를 `MonitoringSubsystem` trait으로 추상화하여 확장성 향상
- FFI 함수 에러 반환 통일 (모두 `Result<T, FfiError>`)
- 데이터베이스 도입 검토 (JSONL → SQLite, 대량 이벤트 쿼리 성능)
- 타입을 별도 `types` 모듈로 분리하여 순환 의존성 예방

### 4.2 코드 품질

- `anyhow` vs `thiserror` 사용 통일 (core는 thiserror만)
- `logger.rs:format_pretty` 함수 복잡도 분리 (이벤트 타입별 포매터)
- `sanitize.rs` `to_lowercase` 반복 호출 캐싱
- dead_code 허용 속성 정리 (`wrapper.rs:436`)
- crate-type에서 불필요한 `staticlib` 제거

### 4.3 문서/인프라

- `README.md` 작성 (프로젝트 개요, 설치, 사용법)
- `cargo audit` 정기 실행으로 취약점 모니터링
- E2E 테스트 스크립트 추가
- 환경별 빌드 프로파일 (`--profile release-prod`)
- Code signing 팀 공유 설정

### 4.4 접근성/국제화

- SwiftUI 동적 타입 크기 지원 (`@ScaledMetric`)
- RTL 언어 지원 테스트
- 접근성 힌트/값 추가 (`.accessibilityHint()`, `.accessibilityValue()`)
- 이모지 애니메이션 비활성화 옵션 (접근성 설정 연동)

---

## 5. 긍정적 사항 — 유지할 패턴

| 영역 | 내용 |
|------|------|
| **보안** | 민감정보 마스킹 시스템이 업계 표준 이상 (`sanitize.rs`). API 키, 토큰, URL 크레덴셜 등 50+ 패턴 커버. 하드코딩된 시크릿 없음 |
| **위험 탐지** | 134개 규칙 기반 위험도 평가 (`risk.rs`). Fork bomb, pipe to shell 등 크리티컬 패턴 감지. Symlink 공격 방어 |
| **아키텍처** | Clean Architecture 레이어 분리 명확. Core ← FFI ← App 의존성 방향 준수. Trait 기반 추상화로 확장성 확보 |
| **타입 안전성** | Rust 타입 시스템 + UniFFI로 Swift와 안전한 통합. Event 타입 owned data로 thread-safe |
| **에러 구조** | thiserror 기반 구조화 에러 (`CoreError`, `ConfigError`, `StorageError`, `FfiError`). FFI 경계 변환 명확 |
| **테스트(Rust)** | 13개 모듈 모두 `#[cfg(test)]` 포함, 300+ 단위 테스트. 엣지 케이스 포함 |
| **MVVM(Swift)** | `@Observable` 매크로로 SwiftUI 통합 우수. ViewModel이 View와 비즈니스 로직 명확히 분리 |
| **로깅** | 3종 포맷 (Pretty/JSON/Compact), 로그 레벨 필터링, 세션 기반 로그 보존 정책 |

---

## 6. 영역별 상세 리뷰

### 6.1 보안 (Security)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🟢 Good | `core/src/sanitize.rs` (전체) | **매우 우수한 민감정보 마스킹 시스템** — API 키, 토큰, 패스워드 등 다양한 패턴 감지. Anthropic, OpenAI, GitHub, AWS, npm 토큰 감지. 대소문자 무관 검색 | 현재 구현 유지 |
| 2 | 🟢 Good | `core/src/detector.rs:97-105` | **민감 디렉토리 패턴 감지** — `.ssh`, `.aws`, `.gnupg`, `.kube` 체크. 심볼릭 링크 우회 방지 (canonicalize) | 현재 구현 유지 |
| 3 | 🟢 Good | `core/src/wrapper.rs:566-567` | **명령어 실행 전 인자 sanitization** — 로깅 전 민감정보 제거 | 현재 구현 유지 |
| 4 | 🟢 Good | `core/src/netmon.rs:320-415` | **`unsafe` 블록 사용이 정당함** — macOS libproc API union 필드 접근에 필요 | 각 unsafe 블록에 `// SAFETY:` 주석 추가 권장 |
| 5 | 🟡 Minor | `core/src/config.rs:187-194` | **기본 민감 파일 패턴이 하드코딩됨** | 향후 사용자 커스터마이징 필요 시 config.toml로 이동 고려 |
| 6 | 🟢 Good | `core/src/risk.rs:158-176` | **위험한 파이프 패턴 감지** — `curl \| bash`, Fork bomb 감지 | 현재 구현 유지 |
| 7 | 🟢 Good | `core/src/error.rs` | **구조화된 에러 타입** — 민감정보 노출 위험 낮음 | 현재 구현 유지 |
| 8 | 🟡 Minor | `core/src/ffi.rs:328-350` | **FFI 레이어에서 파일 읽기 시 경로 검증 없음** | 경로 정규화 및 화이트리스트 체크 추가 |
| 9 | 🟢 Good | `app/.../CoreBridge.swift` | **Swift 레이어는 현재 Mock 데이터 사용** — 실제 FFI 구현 대기 중 | 실제 FFI 연결 시 Rust 보안 규칙 적용됨 |
| 10 | 🟢 Good | 전체 코드베이스 | **하드코딩된 시크릿 없음** | 현재 상태 유지 |

### 6.2 로깅/모니터링 (Logging/Monitoring)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🟢 Good | `core/src/logger.rs:46-55` | **적절한 로그 레벨 분리** — Pretty/JsonLines/Compact 3종 포맷, min_level 필터링 | 현재 구현 유지 |
| 2 | 🟢 Good | `core/src/logger.rs:86-104` | **민감정보 로그 유출 방지** — 로깅 전 sanitize_args 호출 | 현재 구현 유지 |
| 3 | 🟢 Good | `core/src/logger.rs:106-235` | **디버깅 정보 충분** — 타임스탬프, 리스크 이모지, 상세 정보, 색상 코딩 | 현재 구현 유지 |
| 4 | 🟡 Minor | `core/src/logger.rs:198-199` | **JSON 직렬화 실패 시 에러 처리** — fallback은 있으나 로그 누락 가능 | `eprintln!` 추가 |
| 5 | 🟢 Good | `core/src/storage.rs:98-122` | **세션 메타데이터 기록** — session_id, process, pid, timestamp | 현재 구현 유지 |
| 6 | 🟢 Good | `core/src/storage.rs:149-181` | **로그 보존 정책** — 설정 가능한 retention 기간, 자동 정리 | 현재 구현 유지 |
| 7 | 🟢 Good | `core/src/config.rs:133-140` | **Production 디버그 로그 비활성화 가능** — enabled 플래그 | 현재 구현 유지 |
| 8 | 🟢 Good | `core/src/event.rs:156-172` | **alert 플래그 자동 설정** — High/Critical 이벤트 자동 플래그 | 현재 구현 유지 |
| 9 | 🟡 Minor | `core/src/process_tracker.rs:339` | **libproc 에러 무시** — 조용히 빈 벡터 반환 | 디버그 빌드에서만 경고 로그 추가 |
| 10 | 🟢 Good | `cli/src/main.rs:324-341` | **사용자 친화적 출력** — 색상 코딩 배너, 진행 표시 | 현재 구현 유지 |

### 6.3 코드 품질 (Code Quality)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🟡 Minor | `core/src/wrapper.rs:436` | **dead_code 허용 속성** — `process_name` 미사용 | 실제 활용하거나 제거 |
| 2 | 🟡 Minor | `core/src/sanitize.rs:9` | **매직 상수** — MASK 값 "***" 하드코딩 | 표준 패턴이므로 유지 가능 |
| 3 | 🟡 Minor | 여러 파일 | **테스트 커버리지 우수** — 각 모듈에 종합 테스트 | 통합 테스트 추가 고려 |
| 4 | 🟠 Major | `app/.../CoreBridge.swift` | **모든 FFI 함수가 TODO 상태** — mock 데이터 반환 | UniFFI 생성 Swift 바인딩 연결 필요 |
| 5 | 🟡 Minor | `core/src/logger.rs:106-196` | **`format_pretty` Cyclomatic Complexity 높음** | 이벤트 타입별 별도 포매터 함수로 분리 |
| 6 | 🟡 Minor | 전체 Rust 코드 | **`anyhow` 사용 불일치** — 일부 모듈에서만 사용 | `CoreError` 사용으로 통일 권장 |
| 7 | 🟡 Minor | `Cargo.toml:7` | **`edition = "2024"` 불안정** — nightly 전용 | `edition = "2021"` 변경 권장 |

### 6.4 프로젝트 구조 (Project Structure)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🟡 Minor | 프로젝트 루트 | **`README.md` 파일 누락** | 프로젝트 개요, 설치, 사용법 포함 README 작성 |
| 2 | 🟡 Minor | `Cargo.toml:6` | **버전 불일치** — workspace 0.2.0 vs Swift 0.3.0 | 0.3.0으로 통일 권장 |
| 3 | 🟢 Good | 전체 구조 | **Rust core, CLI, Swift app 3-tier 구조 명확** | 유지 |
| 4 | 🟢 Good | `.gitignore` | **포괄적 작성** — Rust, Swift, FFI 산출물 모두 포함 | 유지 |
| 5 | 🟡 Minor | `app/.../` | **중복 생성 파일** — `Generated/` 및 `MacAgentWatchCore/generated/` | 한 곳으로 통일 |
| 6 | 🟢 Good | `core/src/` | **모듈 네이밍 일관성 우수** — 단일 책임 원칙 준수 | 유지 |
| 7 | 🟡 Minor | Swift Views | **`DashboardView.swift` 다소 복잡** | ActivityCards, FilterBar 별도 View 분리 고려 |
| 8 | 🟡 Minor | `app/.../Core/` | **Swift 타입 정의 중복** — FFI 타입 간 수동 동기화 필요 | UniFFI 생성 타입 직접 사용 검토 |
| 9 | 🟢 Good | 전체 | **테스트 구조** — 각 Rust 모듈에 `#[cfg(test)] mod tests` 존재 | 유지 |

### 6.5 아키텍처 설계 (Architecture Design)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🟢 Good | 전체 프로젝트 | **Clean Architecture 레이어 분리 우수** — Core ← FFI ← App 경계 명확 | 유지 |
| 2 | 🟢 Good | `core/src/lib.rs` | **단일 진입점** — 중앙화된 re-export | 유지 |
| 3 | 🟢 Good | `core/src/ffi.rs` | **FFI 경계 설계 우수** — UniFFI 타입 안전성 보장 | 유지 |
| 4 | 🟡 Minor | `core/src/wrapper.rs:191-362` | **Orchestrator 책임 과다** | `MonitoringSubsystem` trait 추상화 고려 |
| 5 | 🟢 Good | `core/src/detector.rs:10-18` | **Detector Trait 설계 우수** — Generic, Clone + Send | 유지 |
| 6 | 🟠 Major | `core/src/ffi.rs:432-505` | **Mutex Lock poisoning 취약** | RwLock 또는 Channel 기반 대체 검토 |
| 7 | 🟢 Good | `app/.../MonitoringViewModel.swift` | **MVVM 패턴 적용 우수** — @Observable 매크로 활용 | 유지 |
| 8 | 🟢 Good | `core/src/storage.rs:13-21` | **EventStorage Trait** — 다양한 백엔드 지원 가능 | 유지 |
| 9 | 🟢 Good | 전체 레이어 | **의존성 방향 준수** — 역방향 의존성 없음 | 유지 |
| 10 | 🟡 Minor | `core/src/ffi.rs:308-430` | **FFI 에러 처리 일관성 부족** — 일부 빈 벡터 반환 | Result 타입으로 통일 |
| 11 | 🟡 Minor | `core/src/netmon.rs:6` | **순환 의존성 가능성** — netmon ↔ detector | `types` 모듈 분리 |
| 12 | 🟠 Major | `core/src/netmon.rs:318-388` | **Unsafe 코드** — libproc union 접근 | safe wrapper 구현 고려 |

### 6.6 의존성 관리 (Dependency Management)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🟡 Minor | `Cargo.toml:15` | **serde 버전 명시 부족** — Major만 지정 | minor 버전 명시 (`"1.0"`) |
| 2 | 🟢 Good | `Cargo.toml:40` | **UniFFI 최신 버전** — `0.29` | 정기 업데이트 확인 |
| 3 | 🟡 Minor | `Cargo.toml:39` | **fsevent 유지보수 상태 확인 필요** | `notify` 크로스 플랫폼 대안 검토 |
| 4 | 🟢 Good | 전체 의존성 | **보안 취약점 없음** | `cargo audit` 정기 실행 |
| 5 | 🟡 Minor | `Cargo.toml:18` | **anyhow + thiserror 중복** — 역할 분리는 명확 | 현재 구조 유지 가능 |
| 6 | 🟢 Good | 의존성 전체 | **불필요한 의존성 없음** | 유지 |
| 7 | 🟢 Good | 전체 | **라이선스 호환성 양호** — MIT/Apache-2.0/MPL-2.0 호환 | `cargo-license` 정기 점검 |
| 8 | 🟢 Good | `Cargo.toml:1-2` | **Workspace resolver = "2"** — 최신 resolver | 유지 |
| 9 | 🟡 Minor | `core/Cargo.toml:10-11` | **crate-type 3종 동시 빌드** — 빌드 시간 증가 | `staticlib` 제거 고려 |
| 10 | 🟠 Major | `libproc`, `fsevent` | **macOS 전용 라이브러리** | 크로스 플랫폼 확장 시 대안 필요 |
| 11 | 🟡 Minor | `core/Cargo.toml` dev-dependencies | **tokio 미사용 가능성** | 실제 사용 여부 확인 후 제거 |

### 6.7 메모리 관리 (Memory Management)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | ~~🔴 Critical~~ 🟢 | `core/src/fswatch.rs` | ~~**stop_flag/thread handle race**~~ | ✅ `Arc<AtomicBool>`로 전환, mutex poisoning 제거 |
| 2 | ~~🔴 Critical~~ 🟢 | `core/src/netmon.rs` | ~~**seen_connections 전체 clear**~~ | ✅ `SeenConnectionsCache` 세대별 캐시로 교체 |
| 3 | ~~🔴 Critical~~ 🟢 | `core/src/wrapper.rs` | ~~**session_logger `Arc<Mutex>` 불필요**~~ | ✅ `Option<Mutex<SessionLogger>>`로 단순화, 안전성 근거 문서화 |
| 4 | 🟠 Major | `core/src/wrapper.rs:451-454` | **stdin thread leak** — `_stdin_handle` join 안됨 | 명시적 join 또는 shutdown signal |
| 5 | 🟠 Major | `core/src/netmon.rs:289-388` | **unsafe union 접근 안전성** — 메모리 레이아웃 불일치 가능 | kind 재확인 방어 코드 추가 |
| 6 | 🟠 Major | `core/src/process_tracker.rs:275-283` | **BFS 큐 무제한 증가** — 프로세스 수천 개 시 | max_depth 기본값 10 설정 |
| 7 | 🟡 Minor | `core/src/storage.rs:71,144-146` | **BufWriter flush 누락** — crash 시 데이터 손실 | auto-flush 옵션 또는 주기적 flush |
| 8 | 🟡 Minor | `app/.../MonitoringViewModel.swift:7-15` | **events 배열 무제한 증가** | 최대 1000개 제한, 페이지네이션 |

### 6.8 동시성/스레드 안전성 (Concurrency/Thread Safety)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | ~~🔴 Critical~~ 🟢 | `core/src/wrapper.rs` | ~~**Orchestrator stop 순서 race**~~ | ✅ 2단계 종료: `signal_stop()` 선행 후 `stop()`+`join()` |
| 2 | ~~🔴 Critical~~ 🟢 | `core/src/ffi.rs` | ~~**FfiMonitoringEngine Mutex 경쟁**~~ | ✅ `SessionState` enum + 원자적 상태 전이 |
| 3 | ~~🔴 Critical~~ 🟢 | `core/src/fswatch.rs` | ~~**FSEvents channel disconnection**~~ | ✅ `catch_unwind` 패턴으로 cleanup 보장 |
| 4 | 🟠 Major | `core/src/netmon.rs:231-286` | **Network monitor busy wait** — interval 부정확 | 정확한 sleep 계산 또는 tokio interval |
| 5 | 🟠 Major | `core/src/wrapper.rs:477-523` | **output_handle/main thread 경쟁** — EOF 전 wait 완료 | join 타임아웃 설정 |
| 6 | 🟠 Major | `core/src/process_tracker.rs:213-252` | **HashMap lock 장기 보유** — reader 블록 | batch 적용, RwLock 고려 |
| 7 | 🟡 Minor | `app/.../MonitoringViewModel.swift:49-57` | **Main actor에서 동기적 FFI 호출** — UI freeze 가능 | `Task.detached` 분리 |
| 8 | 🟡 Minor | `core/src/logger.rs:59-63` | **Logger Clone 시 향후 위험** — 상태 추가 시 | Clone 제거 또는 Arc wrapping |

### 6.9 에러 처리 (Error Handling)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🔴 Critical | `core/src/wrapper.rs:379-381` | **SessionLogger 생성 실패 무시** | 경고 로그 또는 Result 반환 |
| 2 | 🔴 Critical | `core/src/wrapper.rs:247-249` | **FSWatcher 시작 실패 무시** | 에러 로깅 또는 상위 전파 |
| 3 | 🔴 Critical | `core/src/wrapper.rs:286-288` | **NetworkMonitor 시작 실패 무시** | 에러 로깅 또는 상위 전파 |
| 4 | 🟠 Major | `core/src/storage.rs:144-146` | **Drop flush 실패 무시** | `eprintln!` 경고 추가 |
| 5 | 🟠 Major | `core/src/ffi.rs:470-472` | **Lock 실패 메시지 일반적** — "Lock poisoned" | 구체적 메시지로 변경 |
| 6 | 🟠 Major | `core/src/netmon.rs:301` | **listpidinfo 실패 유형 미구분** | ESRCH vs EPERM 구분 |
| 7 | 🟡 Minor | `core/src/process_tracker.rs:190` | **stop_flag lock 실패 시 `unwrap_or(false)`** | 로그 추가 고려 |
| 8 | 🟡 Minor | `core/src/detector.rs:118` | **canonicalize 실패 무시** — 브로큰 심링크 | 원본 경로 기반 체크 유지 (현재 OK) |
| 9 | 🟡 Minor | `cli/src/main.rs:279-281` | **Config 로드 실패 시 `unwrap_or_default()`** | 경고 메시지 출력 추가 |
| 10 | 🟢 Good | `core/src/error.rs` | **구조화된 에러 타입 설계** — FFI 변환 명확 | 유지 |
| 11 | 🟢 Good | `core/src/ffi.rs:287-304` | **CoreError → FfiError 변환 구조적** | 유지 |
| 12 | 🟢 Good | `core/src/storage.rs` | **StorageError에 path + source 포함** | 유지 |

### 6.10 데이터 영속성 (Data Persistence)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🔴 Critical | `core/src/storage.rs:106-107,119` | **header/footer flush 미보장** | write 후 즉시 flush |
| 2 | 🔴 Critical | `core/src/wrapper.rs:598-614` | **session start/end 에러 무시** | eprintln! 추가 |
| 3 | 🟠 Major | `core/src/storage.rs:126-130` | **write_event flush 미호출** — 비정상 종료 시 손실 | N개 이벤트마다 auto-flush |
| 4 | 🟠 Major | 전체 | **DB 미사용** — JSONL 파일만 사용 | SQLite 도입 고려 |
| 5 | 🟠 Major | `core/src/storage.rs:150-181` | **cleanup 삭제 실패 무시** | 경고 로그 및 실패 카운트 반환 |
| 6 | 🟡 Minor | `core/src/ffi.rs:344-346` | **파싱 실패 라인 무시 (skip)** | 경고 로그 또는 에러 카운트 |
| 7 | 🟡 Minor | `core/src/config.rs:38-44` | **첫 실행 시 설정 파일 미생성** | 샘플 config.toml 자동 생성 고려 |
| 8 | 🟡 Minor | `core/src/netmon.rs:256-260` | **seen_connections 전체 clear** — 재탐지 | LRU 또는 시간 기반 제거 |
| 9 | 🟡 Minor | `core/src/storage.rs:51-57` | **세션 ID UUID v4 충돌 가능성** — 극히 낮음 | 현재 충분 |
| 10 | 🟢 Good | `core/src/storage.rs:23-33` | **세션별 로그 파일 격리** | 유지 |
| 11 | 🟢 Good | `core/src/storage.rs:62-69` | **`OpenOptions::append` 안전 추가** | 유지 |
| 12 | 🟢 Good | `core/src/storage.rs:71` | **BufWriter I/O 최적화** | 유지 |
| 13 | 🟢 Good | `core/src/config.rs` | **TOML 기반 설정** — 가독성 우수 | 유지 |
| 14 | 🟢 Good | `core/src/config.rs:75-77` | **플랫폼별 로그 디렉토리** | 유지 |

### 6.11 성능 최적화 (Performance)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🟠 Major | `core/src/process_tracker.rs:269-301` | **get_descendants O(n²)** — 전체 프로세스 순회 | 캐싱 또는 max_depth 엄격 적용 |
| 2 | 🟠 Major | `core/src/netmon.rs:291-388` | **PID당 반복 syscall** — 500ms 폴링으로 CPU 증가 | 1s+ 간격 또는 변경 감지 기반 |
| 3 | 🟠 Major | `core/src/wrapper.rs:498-515` | **line_buffer String push/drain** — 재할당 빈번 | `VecDeque<u8>` 또는 링 버퍼 |
| 4 | 🟡 Minor | `core/src/detector.rs:73-106` | **`to_lowercase` 반복 호출** | 패턴 미리 소문자 변환, lazy_static 캐싱 |
| 5 | 🟡 Minor | `core/src/sanitize.rs:82-133` | **sanitize_args 중복 `to_lowercase`** | 한 번만 변환 후 재사용 |
| 6 | 🟡 Minor | `core/src/storage.rs:126-130` | **매 이벤트 JSON 직렬화** | BufWriter 64KB 확대, 배치 처리 |
| 7 | 🟡 Minor | `core/src/fswatch.rs:176-203` | **FSEvents recv_timeout(100ms)** — CPU 낭비 | latency 500ms 확대 |
| 8 | 🟡 Minor | `app/.../MonitoringViewModel.swift:42-47` | **loadSession 전체 재계산** | 메타데이터 캐싱, 증분 업데이트 |
| 9 | 🟡 Minor | `app/.../DashboardView.swift:149-153` | **filteredEvents 실시간 필터링** | Lazy 필터링, 가상 스크롤 |
| 10 | 🟢 Good | `core/src/risk.rs:75-109` | **RiskScorer 효율적 우선순위 분류** — 조기 종료 | 유지 |
| 11 | 🟢 Good | `core/src/netmon.rs:247-261` | **HashSet 중복 제거 + 메모리 제한** | 유지 |

### 6.12 접근성/국제화 (Accessibility & i18n)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🔴 Critical | Swift UI 전체 | **하드코딩 영어 문자열** — i18n 불가 | `Localizable.strings` + `LocalizedStringKey` |
| 2 | 🔴 Critical | Swift Views 전체 | **접근성 레이블 누락** — VoiceOver 불가 | `.accessibilityLabel()` 전면 추가 |
| 3 | 🔴 Critical | `core/src/event.rs:24-32` | **이모지 하드코딩** | 텍스트 대체 옵션 제공 |
| 4 | 🟠 Major | `cli/src/main.rs:148-174` | **CLI 메시지 영어 고정** | fluent-rs/gettext 도입 |
| 5 | 🟠 Major | `core/src/risk.rs:16` | **RiskRule reason 영어 `&'static str`** | i18n 키로 변경 |
| 6 | 🟠 Major | `app/.../EventRowView.swift:60-77` | **접근성 힌트 누락** | `.accessibilityValue()`, `.accessibilityHint()` |
| 7 | 🟠 Major | `app/.../DashboardView.swift:71-96` | **색상에만 의존한 정보 전달** — 색맹 대응 부족 | 아이콘+텍스트 중복 표시, 고대비 지원 |
| 8 | 🟡 Minor | `app/.../MenuBarView.swift:43-55` | **고정 폰트 크기** — 동적 타입 미지원 | `@ScaledMetric` 사용 |
| 9 | 🟡 Minor | `app/.../SessionListView.swift:26-38` | **날짜 포맷 로케일 미고려** | DateFormatter locale 명시 |
| 10 | 🟡 Minor | `core/src/logger.rs:106-196` | **로그 프리픽스 영어 고정** | 구조화된 로그 필드 분리 |
| 11 | 🟡 Minor | 전체 | **RTL 미지원** | `.environment(\.layoutDirection, .rightToLeft)` 테스트 |
| 12 | 🟢 Good | `app/.../EventRowView.swift:64` | **symbolEffect 사용** | 접근성 설정 연동 권장 |

### 6.13 테스트 커버리지 (Test Coverage)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🔴 Critical | Swift App 전체 | **Swift 테스트 0개** — XCTest 없음 | 테스트 타겟 생성, CoreBridge/ViewModel 테스트 |
| 2 | 🟠 Major | `core/src/fswatch.rs` | **FSEvents 통합 테스트 부족** — macOS 조건부 | FSEvents 모킹 레이어 추가 |
| 3 | 🟠 Major | `core/src/netmon.rs` | **libproc 기반 로직 테스트 없음** — unsafe 포함 | trait 분리 후 mock 테스트 |
| 4 | 🟠 Major | `core/src/wrapper.rs` | **Orchestrator 통합 테스트 없음** | 서브시스템 조합 시나리오 검증 |
| 5 | 🟡 Minor | 전체 | **E2E 테스트 없음** | CLI → 앱 연동 테스트 스크립트 |
| 6 | 🟢 Good | `core/src/` 전체 | **Rust core 300+ 단위 테스트** — 엣지 케이스 포함 | 유지 |

### 6.14 CI/CD/빌드 설정 (Build Configuration)

| # | 심각도 | 파일/위치 | 설명 | 권장 조치 |
|---|--------|-----------|------|-----------|
| 1 | 🔴 Critical | 프로젝트 루트 | **CI 파이프라인 없음** | GitHub Actions: test.yml, build.yml |
| 2 | 🔴 Critical | 프로젝트 루트 | **Makefile 없음** | make test/build/build-ffi/clean 정의 |
| 3 | 🟠 Major | `Cargo.toml:7` | **`edition = "2024"` 불안정** | `"2021"` 변경 |
| 4 | 🟠 Major | `scripts/build-ffi.sh` | **의존성 검증 없음** — uniffi-bindgen 등 | 필수 도구 체크 로직 추가 |
| 5 | 🟠 Major | `core/Cargo.toml:10-11` | **crate-type 3종 동시 빌드** | staticlib 제거로 시간 단축 |
| 6 | 🟡 Minor | `core/Cargo.toml` | **dev-dependencies tokio 미사용 가능** | 확인 후 제거 |
| 7 | 🟡 Minor | 전체 | **환경 분리 없음** — DEV/STAGING/PROD | 빌드 프로파일 추가 권장 |
| 8 | 🟡 Minor | Xcode 프로젝트 | **Code signing 팀 공유 설정 부재** | project.pbxproj 또는 환경변수 관리 |
| 9 | 🟢 Good | `.gitignore` | **잘 구성됨** — 빌드 아티팩트, 민감정보 제외 | 유지 |
| 10 | 🟢 Good | `scripts/build-ffi.sh` | **UniFFI 빌드 체계적** — `set -euo pipefail` | 유지 |

---

## 7. 최종 결론

**MacAgentWatch는 전반적으로 잘 설계되고 구현된 고품질 코드베이스입니다.**

### 핵심 강점
- 보안 설계 (sanitize, detector, risk)
- Trait 기반 Clean Architecture
- Rust 테스트 커버리지 300+
- 타입 안전성 (Rust + UniFFI)

### 핵심 약점
- ~~CI/CD 파이프라인 부재~~ ✅ 해결됨
- ~~Swift 테스트 0개~~ ✅ 71개 테스트 추가
- ~~동시성/메모리 관련 race condition 6건~~ ✅ 6건 모두 해결됨
- 접근성/i18n 미지원

### 프로덕션 배포 판단

> **🔴 Critical ~~17건~~ → ~~14건~~ → 8건 해결 필요** (9건 조치 완료)
> - ~~C1-C3: 인프라 (CI, Makefile, Swift 테스트)~~ ✅ 조치 완료
> - ~~C4-C9: 동시성/메모리 안전성~~ ✅ 조치 완료
> - C10-C12: 에러 처리 (조용한 실패 방지)
> - C13-C14: 데이터 영속성 (flush 보장)
> - C15-C17: 접근성/국제화

다음 우선 과제: 에러 처리 (C10-C12) 및 데이터 영속성 (C13-C14) 개선.
