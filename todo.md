# MacAgentWatch 개발 TODO

> PRD 로드맵 기반 개발 태스크 목록

---

## Phase 1: Core + CLI (v0.1 - v0.2) - ✅ 완료

### v0.1.0 (MVP) - ✅ 완료
- [x] Rust 코어 라이브러리 기본 구조 설정
  - [x] `core/` 디렉토리 및 Cargo.toml 생성
  - [x] 기본 모듈 구조 (`lib.rs`, `wrapper.rs`, `risk.rs`, `logger.rs`, `event.rs`)
- [x] 프로세스 래퍼 구현 (`wrapper.rs`)
  - [x] 자식 프로세스 생성 및 관리 (PTY 지원)
  - [x] stdin/stdout/stderr 스트림 캡처
  - [x] portable-pty를 사용한 PTY 기반 래핑
  - [x] fork/exec 추적 (libproc 연동 완료)
- [x] 자식 프로세스 추적
  - [x] libproc을 사용한 프로세스 트리 모니터링
  - [x] ProcessTracker 모듈 (`process_tracker.rs`)
  - [x] 100ms 폴링 간격으로 자식 프로세스 감지
  - [x] 자식 프로세스 리스크 스코어링 적용
  - [x] CLI 옵션: `--no-track-children`, `--tracking-poll-ms`
- [x] 명령어 로깅
  - [x] 실행된 명령어 기록
  - [x] 타임스탬프 및 PID 포함
  - [x] 이벤트 기반 구조화된 로깅 (`event.rs`)
- [x] 기본 리스크 스코어링 (`risk.rs`)
  - [x] 리스크 레벨 정의 (Critical, High, Medium, Low)
  - [x] 파괴적 명령어 탐지 (`rm -rf`, `chmod 777`, fork bomb 등)
  - [x] 권한 상승 명령어 탐지 (`sudo`, `su` 등)
  - [x] 위험한 파이프 패턴 탐지 (`curl | bash` 등)
  - [x] 커스텀 고위험 명령어 추가 기능
- [x] CLI 출력
  - [x] `cli/` 디렉토리 및 기본 구조
  - [x] clap을 사용한 CLI 인터페이스
  - [x] 컬러 출력 및 리스크 레벨 표시
  - [x] 다중 출력 포맷 지원 (Pretty, JSON, Compact)
  - [x] `analyze` 서브커맨드 (명령어 위험도 분석)
  - [x] 헤드리스 모드 지원

### v0.2.0 - ✅ 완료
- [x] FSEvents 파일 모니터링 (`fswatch.rs`)
  - [x] fsevent 크레이트 연동
  - [x] 파일 읽기/쓰기/삭제/권한변경 이벤트 캡처
  - [x] 100ms 이벤트 지연 시간 설정
  - [x] 백그라운드 스레드 실행
- [x] 민감 파일 탐지 (`detector.rs`)
  - [x] `.env`, `.env.*` 패턴 탐지
  - [x] `*credentials*`, `*secret*` 패턴 탐지
  - [x] `~/.ssh/*`, `~/.aws/*` 경로 감시
  - [x] `*.pem`, `*.key` 파일 접근 알림
  - [x] 사용자 정의 패턴 지원
  - [x] Detector trait 기반 추상화
- [x] JSON Lines 로깅 (`storage.rs`)
  - [x] 구조화된 JSON 로그 포맷
  - [x] 세션별 로그 파일 저장 (`session-{날짜}-{UUID}.jsonl`)
  - [x] `~/.macagentwatch/logs/` 저장
  - [x] 버퍼링된 쓰기
  - [x] 30일 기본 보존 정책
- [x] 설정 파일 지원 (`config.rs`)
  - [x] `~/.macagentwatch/config.toml` 파싱
  - [x] 민감 파일 패턴 설정
  - [x] 네트워크 화이트리스트 설정
  - [x] 로깅 설정 (포맷, 레벨, 색상, 타임스탬프)
  - [x] 모니터링 설정 (경로, 폴링 간격)
  - [x] 알림 설정
- [x] 네트워크 모니터링 (`netmon.rs`)
  - [x] libproc 기반 연결 추적
  - [x] TCP/UDP 연결 모니터링
  - [x] 도메인/IP/포트 로깅
  - [x] NetworkWhitelist로 알려진 서비스 허용
  - [x] 500ms 폴링 간격
- [x] CLI 확장
  - [x] `--watch <path>` 파일 감시 경로
  - [x] `--enable-fswatch` FSEvents 활성화
  - [x] `--enable-netmon` 네트워크 모니터링 활성화
  - [x] `--log-dir <path>` 로그 디렉토리 설정
  - [x] `--config <path>` 설정 파일 경로

### Phase 1 코드 리뷰 후 추가 작업 - ✅ 완료
- [x] 네트워크 모니터링 실제 구현 완료 (`netmon.rs`)
  - [x] `get_connections_for_pid()` 함수에서 실제 소켓 연결 정보 추출
  - [x] libproc의 `pidfdinfo`와 `SocketFDInfo` 사용하여 원격 주소 획득
  - [x] TCP/UDP 연결 상태 추적 (Established, SynSent, SynReceived)
  - [x] 루프백 주소 필터링 (127.0.0.1, ::1)
  - [x] 중복 연결 제거
- [x] 버전 번호 업데이트
  - [x] `Cargo.toml`의 `version`을 `"0.2.0"`으로 변경
- [x] 보안 강화 (`sanitize.rs`)
  - [x] API 키 마스킹 (Anthropic, OpenAI, GitHub, AWS, npm)
  - [x] 환경 변수 마스킹 (`API_KEY=`, `SECRET=`, `TOKEN=`)
  - [x] 민감한 명령어 인수 마스킹 (`-p`, `--password`, `--token`, `--api-key`)
  - [x] Bearer 토큰 마스킹
  - [x] HTTP 헤더 마스킹 (`Authorization:`, `X-Api-Key:`)
  - [x] 65+ 테스트 케이스
- [x] 심볼릭 링크 해석 (`detector.rs`에서 `canonicalize()` 사용)

---

## Phase 2: macOS 네이티브 앱 (v0.3 - v0.5)

### v0.3.0 - ✅ 완료

#### Rust FFI 레이어 - ✅ 완료
- [x] UniFFI 기반 FFI 구현 (`ffi.rs`, 900줄, 48 테스트)
  - [x] UniFFI 0.29 스캐폴딩 (`lib.rs`에서 `uniffi::setup_scaffolding!()`)
  - [x] FFI-safe 타입 정의 (`FfiEvent`, `FfiConfig`, `FfiActivitySummary`, `FfiSessionInfo`)
  - [x] FFI-safe 열거형 (`FfiRiskLevel`, `FfiFileAction`, `FfiProcessAction`, `FfiSessionAction`, `FfiEventType`)
  - [x] FFI-safe 에러 타입 (`FfiError`: Config, Storage, Io, Other)
  - [x] Core→FFI 타입 변환 (`Event→FfiEvent`, `Config→FfiConfig`, `CoreError→FfiError`)
  - [x] 내보내기 함수: `load_config()`, `analyze_command()`, `get_version()`, `read_session_log()`, `list_session_logs()`, `get_activity_summary()`
  - [x] `FfiMonitoringEngine` 객체 (세션 시작/중지/활성 상태 확인)
- [x] 정적 라이브러리 빌드 (`libmacagentwatch_core.a`, `libmacagentwatch_core.dylib`)
  - [x] `Cargo.toml`: `crate-type = ["staticlib", "cdylib", "lib"]`
  - [x] `uniffi-bindgen.rs` 바이너리 엔트리포인트
- [x] FFI 빌드 스크립트 (`scripts/build-ffi.sh`)
  - [x] Rust 정적 라이브러리 빌드 (debug/release)
  - [x] UniFFI Swift 바인딩 자동 생성
  - [x] 아티팩트 복사 (`lib/`, `include/`, `Generated/`)
  - [x] `module.modulemap` 생성

#### SwiftUI macOS 앱 - ✅ 완료 (실제 FFI 연동)
- [x] Xcode 프로젝트 생성 (`app/MacAgentWatch/`)
  - [x] macOS 14.0+ 타겟
  - [x] Swift 5.0
  - [x] App Sandbox 비활성화 (시스템 모니터링용)
  - [x] Rust 정적 라이브러리 링크 설정 (`libmacagentwatch_core.a`)
  - [x] `Security.framework`, `SystemConfiguration.framework`, `libresolv` 연결
- [x] 앱 구조
  - [x] `MacAgentWatchApp.swift`: `@main` 진입점, `MenuBarExtra` + `WindowGroup`
  - [x] `MonitoringViewModel.swift`: `@Observable` 중앙 상태 관리
  - [x] `CoreBridge.swift`: FFI 브릿지 싱글턴 (실제 FFI 호출 + 에러 시 폴백)
  - [x] `MonitoringTypes.swift`: Swift 데이터 모델 (`RiskLevel`, `EventType`, `MonitoringEvent`, `ActivitySummary`, `SessionInfo`)
  - [x] `ConfigTypes.swift`: 설정 모델 (`GeneralConfig`, `LoggingConfig`, `MonitoringConfig`, `AlertConfig`)
- [x] 메뉴바 상주 기능 (`MenuBarView.swift`)
  - [x] `MenuBarExtra` API (macOS 13+, `.window` 스타일 팝오버)
  - [x] 시스템 아이콘 (`shield.checkered`)
  - [x] 앱 이름, 버전, 상태 배지 (Active/Idle)
  - [x] 활동 요약 카드 (Total, Critical, High, Medium)
  - [x] 최근 알림 리스트 (최대 5개)
  - [x] "Open Dashboard", "Quit" 액션
- [x] 대시보드 뷰 (`DashboardView.swift`)
  - [x] `NavigationSplitView` 아키텍처
  - [x] 활동 카드 5개 (Total, Critical, High, Medium, Low)
  - [x] 리스크 레벨 필터 바 (All, Low, Medium, High, Critical)
  - [x] 이벤트 리스트 (필터링 가능)
  - [x] 최소 크기 800x500
- [x] 이벤트 행 뷰 (`EventRowView.swift`)
  - [x] 리스크 표시기 (컬러 원형)
  - [x] 이벤트 아이콘 (terminal, doc, network, gearshape)
  - [x] 명령어/파일/네트워크 상세 표시
  - [x] 프로세스 이름, PID, 상대 시간 표시
- [x] 세션 리스트 뷰 (`SessionListView.swift`)
  - [x] 세션 ID (축약), 타임스탬프
  - [x] 선택 상태 하이라이트
  - [x] 세션 클릭 시 로드
- [x] UniFFI 생성 Swift 바인딩 (`Generated/macagentwatch_core.swift`)

#### FFI 연동 및 엔진 통합 - ✅ 완료
- [x] CoreBridge에서 실제 FFI 호출 연동 (6개 함수 + 엔진 3개 함수)
  - [x] `loadConfig()` → `macagentwatch_core.loadConfig()` 연결
  - [x] `analyzeCommand()` → `macagentwatch_core.analyzeCommand()` 연결
  - [x] `getVersion()` → `macagentwatch_core.getVersion()` 연결
  - [x] `readSessionLog()` → `macagentwatch_core.readSessionLog()` 연결
  - [x] `listSessionLogs()` → `macagentwatch_core.listSessionLogs()` 연결
  - [x] `getActivitySummary()` → `macagentwatch_core.getActivitySummary()` 연결
- [x] Swift 타입 ↔ FFI 타입 변환 레이어 구현
  - [x] `MonitoringEvent` ↔ `FfiEvent` 변환 (양방향)
  - [x] `AppConfig` ↔ `FfiConfig` 변환
  - [x] `SessionInfo` ↔ `FfiSessionInfo` 변환
  - [x] `ActivitySummary` ↔ `FfiActivitySummary` 변환
- [x] `FfiMonitoringEngine` 연동
  - [x] CoreBridge에서 `FfiMonitoringEngine` 인스턴스 관리 (지연 생성)
  - [x] `startSession()` / `stopSession()` / `isEngineActive()` 메서드
  - [x] ViewModel `startMonitoring()` → `bridge.startSession("MacAgentWatch")`
  - [x] ViewModel `stopMonitoring()` → `bridge.stopSession()`
  - [x] `isMonitoring` 상태를 `engine.isActive()`와 동기화
  - [x] `currentSessionId` 프로퍼티로 활성 세션 추적
- [x] FFI 에러 핸들링
  - [x] 모든 FFI 함수에 try/catch 에러 핸들링
  - [x] 실패 시 목 데이터/빈 상태로 우아한 폴백
  - [x] ViewModel `errorMessage` 프로퍼티로 사용자 피드백
- [x] 버전 번호: `Cargo.toml` 워크스페이스 버전 `0.3.0`
- [x] Swift 테스트 업데이트 (실제 FFI 기반, 새 테스트 7개 추가)

### v0.4.0 - ✅ 완료

#### Rust FFI 확장 - ✅ 완료
- [x] `parse_events_from_file()` 공통 파싱 헬퍼
- [x] `read_session_log_paginated()` 페이지네이션 지원
- [x] `get_session_event_count()` 이벤트 수 조회
- [x] `get_chart_data()` 시간대별 차트 데이터 집계
- [x] `search_events()` 텍스트 검색 + 타입/리스크/시간 필터
- [x] `get_latest_events()` 라이브 폴링용 새 이벤트 조회
- [x] `FfiChartDataPoint` FFI 레코드 타입 추가
- [x] 22+ 신규 테스트 (전체 242개 통과)

#### 풀 대시보드 UI 강화 - ✅ 완료
- [x] Activity Overview 섹션 (v0.3.0에서 기본 구현 완료)
- [x] Risk Events 리스트 (v0.3.0에서 기본 구현 완료)
- [x] 세션 사이드바 (v0.3.0에서 기본 구현 완료)
- [x] 이벤트 상세 보기 (`EventDetailView.swift` - inspector 패널, 311줄)
  - [x] EventType별 상세 정보 (Command/File/Network/Process/Session)
  - [x] 클립보드 복사 기능 (이벤트 ID, 명령어, 경로)
  - [x] VoiceOver/Dynamic Type/High Contrast 접근성
- [x] 대시보드 탭 전환 (Events / Live Log / Charts)
  - [x] `DetailTab` enum + Segmented Picker
- [x] 이벤트 선택 → inspector 패널 표시

#### 실시간 로그 뷰어 - ✅ 완료
- [x] `LiveLogView.swift` (259줄)
- [x] Timer 기반 1초 폴링 라이브 스트리밍
- [x] 리스크 레벨별 색상 표시
- [x] 자동 스크롤 (ScrollViewReader + scrollTo)
- [x] 일시 정지/재개 토글
- [x] 로그 지우기 기능
- [x] 최대 500 라인 FIFO
- [x] 모노스페이스 폰트 로그 라인

#### 필터링 및 검색 강화 - ✅ 완료
- [x] 리스크 레벨 필터 (v0.3.0에서 기본 구현 완료)
- [x] 텍스트 검색 (300ms 디바운스, CMD+F 단축키 지원)
- [x] 날짜별 필터 (All Time/Today/Last Hour/24h/7Days/Custom)
- [x] 이벤트 타입별 필터 (All/Command/FileAccess/Network/Process)
- [x] `EventTypeFilter`, `DateRangePreset` 열거형 추가
- [x] ViewModel `filteredEvents` 다중 필터 통합

#### Swift Charts 통계 - ✅ 완료
- [x] `ChartsView.swift` (229줄)
- [x] 활동 타임라인 (BarMark - 리스크 레벨별 스택)
- [x] 리스크 분포 (SectorMark - 도넛 차트)
- [x] 이벤트 타입 분포 (수평 BarMark)
- [x] 기간 선택 (24h/7d/30d)
- [x] 리스크 레벨별 커스텀 색상 매핑

#### CoreBridge + ViewModel 연동 - ✅ 완료
- [x] CoreBridge 5개 신규 FFI 함수 연동
- [x] `FfiChartDataPoint` → `ChartDataPoint` 변환
- [x] ViewModel `chartData`, `liveEventIndex` 프로퍼티
- [x] `loadChartData()`, `pollLatestEvents()` 메서드
- [x] 로컬라이제이션 35+ 신규 키 추가

### v0.5.0
- [ ] macOS 네이티브 알림
  - [ ] UserNotifications 연동
  - [ ] Critical/High 이벤트 알림
- [ ] 설정 화면
  - [ ] 민감 파일 패턴 관리
  - [ ] 알림 설정
  - [ ] 동기화 설정
- [ ] 다크 모드 지원
  - [ ] 시스템 테마 따르기
  - [ ] 커스텀 컬러 스킴
- [ ] 키보드 단축키
  - [ ] 글로벌 단축키 설정
  - [ ] 대시보드 내 네비게이션

---

## Phase 3: iOS 앱 + 동기화 (v0.6 - v0.8)

### v0.6.0
- [ ] CloudKit 동기화 구현
  - [ ] CloudKit 컨테이너 설정
  - [ ] 로그 데이터 동기화
  - [ ] 설정 동기화
- [ ] iOS 앱 기본 구조
  - [ ] iOS 타겟 추가
  - [ ] 공유 코드 활용
- [ ] 로그 뷰어 (읽기 전용)
  - [ ] Mac 로그 조회
  - [ ] 필터링 및 검색

### v0.7.0
- [ ] 푸시 알림 (APNs)
  - [ ] APNs 설정
  - [ ] 위험 이벤트 푸시
  - [ ] 알림 카테고리 및 액션
- [ ] 실시간 상태 확인
  - [ ] Mac 연결 상태
  - [ ] 현재 실행 중인 에이전트
- [ ] iPad 레이아웃 최적화
  - [ ] 멀티 컬럼 레이아웃
  - [ ] 사이드바 네비게이션

### v0.8.0
- [ ] 로컬 네트워크 모드
  - [ ] Network.framework P2P 통신
  - [ ] iCloud 없이 직접 연결
- [ ] 위젯 (iOS/macOS)
  - [ ] 상태 요약 위젯
  - [ ] 최근 알림 위젯
- [ ] Apple Watch 알림 (선택적)
  - [ ] watchOS 타겟
  - [ ] 간단한 알림 수신

---

## Phase 4: 안정화 + 출시 (v0.9 - v1.0)

### v0.9.0
- [ ] 성능 최적화
  - [ ] CPU 사용량 < 5% 목표
  - [ ] 메모리 사용량 < 50MB (Core)
  - [ ] 앱 크기 최적화 (macOS < 30MB, iOS < 15MB)
- [ ] 버그 수정
  - [ ] 엣지 케이스 처리
  - [ ] 메모리 누수 점검
- [ ] 베타 테스트
  - [ ] TestFlight 배포
  - [ ] 피드백 수집 및 반영

### v1.0.0
- [ ] App Store 출시
  - [ ] macOS App Store 제출
  - [ ] iOS App Store 제출
  - [ ] 앱 스크린샷 및 설명
- [ ] Homebrew 배포 (CLI)
  - [ ] Homebrew formula 작성
  - [ ] homebrew-core PR
- [ ] 문서화 완료
  - [ ] README.md 작성
  - [ ] docs/architecture.md
  - [ ] docs/development.md
  - [ ] docs/api.md

---

## Future (v2.0+)

- [ ] 행동 차단 모드 (선택적)
- [ ] AI 기반 이상 탐지
- [ ] 멀티 Mac 지원
- [ ] 팀 협업 기능 (공유 대시보드)
- [ ] Shortcuts 앱 연동

---

## 참고사항

### 성공 지표
| 지표 | 목표 |
|------|------|
| 설치 편의성 | App Store에서 1분 내 설치 |
| 성능 오버헤드 | CPU < 5%, 메모리 < 50MB |
| 앱 크기 | macOS < 30MB, iOS < 15MB |
| 탐지 정확도 | 민감 파일 접근 100% 탐지 |
| 알림 지연 | 위험 이벤트 → 푸시 알림 < 3초 |

### 기술 스택

**Core (v0.2.0 완료)**
| 구성 요소 | 기술 | 상태 |
|-----------|------|------|
| 언어 | Rust (edition 2024) | ✅ |
| PTY 래핑 | portable-pty 0.8 | ✅ |
| 파일 감시 | fsevent 2 (FSEvents API) | ✅ |
| 프로세스 추적 | libproc 0.14 | ✅ |
| 네트워크 추적 | libproc 0.14 | ✅ |
| 직렬화 | serde 1 + serde_json 1 | ✅ |
| 설정 파일 | toml 0.8 + dirs 5 | ✅ |
| 패턴 매칭 | glob 0.3 | ✅ |
| CLI | clap 4 (derive) | ✅ |
| 시간 | chrono 0.4 | ✅ |
| 컬러 출력 | colored 3 | ✅ |
| 에러 처리 | thiserror 2 + anyhow 1 | ✅ |
| UUID | uuid 1 (v4, serde) | ✅ |
| FFI | UniFFI 0.29 | ✅ |

**App (v0.4.0 완료)**
| 구성 요소 | 기술 | 상태 |
|-----------|------|------|
| UI 프레임워크 | SwiftUI (macOS 14+) | ✅ |
| 상태 관리 | @Observable (Swift 5.9+) | ✅ |
| 메뉴바 | MenuBarExtra API | ✅ |
| FFI 바인딩 | UniFFI 생성 Swift 코드 | ✅ |
| FFI 실제 연결 | CoreBridge → FFI 호출 (11개 함수) | ✅ |
| Swift Charts | 활동 타임라인/리스크 분포/이벤트 타입 | ✅ |
| CloudKit | (v0.6.0 예정) | ❌ |
| Network.framework | (v0.8.0 예정) | ❌ |

---

## 현재 구현 상태 요약

### 빌드 & 테스트 현황 (2026-02-07 기준)
- Rust 빌드: ✅ 성공 (Cargo workspace: `core`, `cli`)
- Rust 테스트: ✅ **257개 통과** (core 242 + cli 15)
- Rust clippy: ✅ 경고 0개
- 워크스페이스 버전: `0.4.0`
- Rust edition: `2021`

### 완료된 기능 (v0.1.0 + v0.2.0 + v0.3.0 + v0.4.0)

**Core 라이브러리** (`macagentwatch-core`, ~9,650줄)
| 모듈 | 설명 | 줄 수 | 테스트 |
|------|------|-------|--------|
| `lib.rs` | 공개 API 내보내기, UniFFI 스캐폴딩 | 62 | 2 (doc) |
| `event.rs` | 이벤트 타입 (Command, FileAccess, Network, Process, Session) | 343 | 43 |
| `wrapper.rs` | PTY 프로세스 래퍼 + MonitoringOrchestrator | 796 | 18 |
| `risk.rs` | 리스크 스코어링 (27개 규칙, 패턴 매칭) | 418 | 17 |
| `logger.rs` | 다중 포맷 로거 (Pretty/JSON/Compact) | ~200 | - |
| `process_tracker.rs` | libproc 기반 자식 프로세스 추적 | ~300 | - |
| `fswatch.rs` | FSEvents 파일 모니터링 | ~250 | - |
| `netmon.rs` | libproc 기반 TCP/UDP 네트워크 연결 추적 | ~300 | - |
| `detector.rs` | 민감 데이터 탐지 (26개 기본 패턴, Detector trait) | 538 | 22 |
| `config.rs` | TOML 설정 파일 파싱 | 402 | 17 |
| `storage.rs` | JSON Lines 세션 로깅 | 379 | 13 |
| `sanitize.rs` | 자격 증명 마스킹 (API 키, 토큰, 비밀번호) | ~300 | 65+ |
| `error.rs` | 구조화된 에러 타입 (thiserror 기반) | 110 | - |
| `ffi.rs` | UniFFI FFI 레이어 (11 함수, 타입 변환) | ~1,510 | 70+ |

**CLI 애플리케이션** (`macagentwatch`, ~400줄)
- `macagentwatch -- <command>` : 프로세스 래핑 및 모니터링
- `macagentwatch analyze <command>` : 명령어 위험도 분석
- `macagentwatch version` : 버전 정보

**CLI 옵션**
| 옵션 | 설명 |
|------|------|
| `--format pretty\|json\|compact` | 출력 포맷 |
| `--min-level low\|medium\|high\|critical` | 최소 리스크 레벨 |
| `--no-color` | 컬러 비활성화 |
| `--no-timestamps` | 타임스탬프 숨김 |
| `--watch <path>` | 파일 감시 경로 |
| `--headless` | 헤드리스 모드 |
| `--no-track-children` | 자식 프로세스 추적 비활성화 |
| `--tracking-poll-ms <ms>` | 추적 폴링 간격 |
| `--enable-fswatch` | FSEvents 활성화 |
| `--enable-netmon` | 네트워크 모니터링 활성화 |
| `--log-dir <path>` | 로그 디렉토리 |
| `--config <path>` | 설정 파일 경로 |

**macOS 네이티브 앱** (`app/MacAgentWatch/`, SwiftUI)
| 파일 | 설명 | 상태 |
|------|------|------|
| `MacAgentWatchApp.swift` | 앱 진입점 (`MenuBarExtra` + `WindowGroup`) | ✅ |
| `CoreBridge.swift` | FFI 브릿지 싱글턴 (11개 FFI 함수 + 엔진 관리) | ✅ |
| `MonitoringTypes.swift` | Swift 데이터 모델 (191줄, 11 타입) | ✅ |
| `ConfigTypes.swift` | 설정 모델 | ✅ |
| `MonitoringViewModel.swift` | `@Observable` 상태 관리 (165줄, 다중 필터) | ✅ |
| `MenuBarView.swift` | 메뉴바 팝오버 UI | ✅ |
| `DashboardView.swift` | 대시보드 + 탭 전환 (Events/LiveLog/Charts) | ✅ |
| `EventRowView.swift` | 이벤트 행 표시 | ✅ |
| `EventDetailView.swift` | 이벤트 상세 inspector 패널 (311줄) | ✅ NEW |
| `LiveLogView.swift` | 실시간 로그 뷰어 (259줄) | ✅ NEW |
| `ChartsView.swift` | Swift Charts 통계 (229줄) | ✅ NEW |
| `FilterBarView.swift` | 강화된 필터 (검색+날짜+타입, ~280줄) | ✅ |
| `SessionListView.swift` | 세션 사이드바 | ✅ |
| `Generated/macagentwatch_core.swift` | UniFFI 생성 바인딩 | ✅ |

### 다음 단계 (v0.5.0)
- [ ] macOS 네이티브 알림 (UserNotifications, Critical/High 이벤트)
- [ ] 설정 화면 UI (민감 파일 패턴, 알림, 동기화)
- [ ] 다크 모드 지원 (시스템 테마 따르기)
- [ ] 키보드 단축키 (글로벌 + 대시보드 네비게이션)

### 미구현 (v0.6.0+)
- [ ] CloudKit 동기화 (v0.6.0)
- [ ] iOS 앱 (v0.6.0)
- [ ] 푸시 알림 APNs (v0.7.0)
- [ ] 위젯 iOS/macOS (v0.8.0)
- [ ] App Store 출시 (v1.0.0)

### 아키텍처 레이어
```
┌────────────────────────────────────────────────────┐
│     Swift macOS App (SwiftUI)                      │
│  MenuBarView ← MonitoringViewModel                 │
│  DashboardView (탭: Events / LiveLog / Charts)     │
│  EventDetailView, EventRowView, ChartsView         │
│  FilterBarView (검색+날짜+타입+리스크), SessionList │
└──────────────────┬─────────────────────────────────┘
                   │ CoreBridge (11개 FFI 함수 + FfiMonitoringEngine)
┌──────────────────▼─────────────────────────────────┐
│         UniFFI FFI Layer (ffi.rs)                  │
│  FfiMonitoringEngine + 내보내기 함수 11개           │
│  페이지네이션, 차트 집계, 검색, 라이브 폴링         │
│  타입 변환: Event→FfiEvent, Config→FfiConfig       │
└──────────────────┬─────────────────────────────────┘
                   │
┌──────────────────▼─────────────────────────────────┐
│        Rust Core Library (16개 모듈)               │
│  ProcessWrapper → Orchestrator                     │
│  ├── ProcessTracker (libproc)                      │
│  ├── FileSystemWatcher (FSEvents)                  │
│  └── NetworkMonitor (libproc TCP/UDP)              │
│  RiskScorer (27규칙) + Detector (26패턴)            │
│  SessionLogger (JSONL) + Sanitizer                 │
└────────────────────────────────────────────────────┘
```