# tauri-sql — 컨텍스트

## 프로젝트 요약
macOS 에서 가볍게 쓰는 MSSQL 쿼리/탐색 도구.
Tauri 2 + SvelteKit (Svelte 5) + Rust(tiberius). **최우선 가치: 안정성**.

## 스택
- Tauri 2 + SvelteKit (Svelte 5 runes) + Tailwind v4 + TypeScript
- Rust: tokio, tiberius 0.12 + bb8-tiberius 0.16 풀(size 4), keyring v4, rust_xlsxwriter, thiserror
- Frontend: AG Grid Community 32 (가상 스크롤), CodeMirror 6 + lang-sql, @tauri-apps/plugin-dialog

## 핵심 설계
- 세션 = 프로필 1개의 bb8 풀. `Arc<RwLock<HashMap<Uuid, Session>>>` State.
- 쿼리 취소 = `tokio::select! biased` 의 cancel branch → PooledConnection drop → pool 의 `test_on_check_out` 이 다음 사용 시 검증
- TOP N 보호: row stream 을 max_rows 행에서 break, `truncated` 플래그
- 비밀번호: keyring v4 (macOS Keychain). profiles.json 은 host/port/db/user 만
- SQL 분류기(sql_safety.rs): ReadOnly / Write / Ddl / Unknown → 후자 셋은 프론트에서 확인 다이얼로그
- 익스플로러는 lazy: sys 카탈로그만 사용, DB 이름은 QUOTENAME escape, 시스템 DB(master/msdb/tempdb) 제외
- 결과 직렬화: `RowValue` enum (`Int`/`Float`/`Decimal(String)`/`Text`/`DateTime(ISO8601)`/`Uuid`/`Binary(base64)` 등)
- xlsx export: Rust 가 RowValue 받아 타입 보존하여 셀 작성 (`rust_xlsxwriter::Worksheet::autofit()`)
- 종료 cleanup: `RunEvent::ExitRequested` 에서 `QueryRegistry::cancel_all()` → `SessionManager::close_all()` (3초 timeout)

## 진행 상태
- [x] Phase 0: 스캐폴딩
- [x] Phase 1: 연결 프로필 CRUD + keyring + test_connection
- [x] Phase 2: 세션 풀 + 기본 쿼리
- [x] Phase 3: AG Grid + CodeMirror
- [x] Phase 4: 안전장치 (sql_safety 9개 테스트, cancel, timeout, max_rows, 확인 다이얼로그)
- [x] Phase 5: 오브젝트 익스플로러 lazy
- [x] Phase 6: 마감 (단축키, README)
- [x] 후속: 정렬 옵션화 + autoSize, 드래그 splitter, 다크모드, SQL 키워드 색, 파일 IO
- [x] 후속2: 그리드 행/셀 ⌘C 복사, Excel 내보내기
- [x] 후속3: 실행 중 오버레이, 선택 영역만 실행
- [x] 후속4: 시스템 DB 숨김, graceful 종료, 아이콘 등록
- [x] 후속5: 다중 쿼리 탭(독립 실행), Object Explorer 폭 드래그(localStorage 저장)

## tiberius 핵심 API (context7 조회)
- `Config::from_ado_string("server=tcp:host,port;database=db;user id=u;password=p;TrustServerCertificate=true")`
- 연결: `Client::connect(config, tcp.compat_write()).await?`
- 쿼리: `client.simple_query(sql).await?` → `QueryStream`
- 스트리밍: `into_row_stream()` + `futures_util::TryStreamExt::try_next`
- 메타: `QueryItem::Metadata(meta)` / `QueryItem::Row(row)`

## 주의사항/함정
- ag-grid-svelte5 패키지가 npm 에 없음 → vanilla `createGrid()` 직접 mount
- tiberius cancel 미지원 — `tokio::select!` + PooledConnection drop + `test_on_check_out` 으로 우회
- Decimal/Money: `rust_decimal::Decimal` → 문자열 (정밀도 보존)
- DB 이름은 sys 카탈로그 결과 + `]→]]` escape (인젝션 차단)
- macOS 에서 Windows AD 인증 어려움 — SQL Server 인증만 1차 지원
- **bb8-tiberius default features 가 winauth 켜기 때문에 macOS 빌드 실패** → `default-features=false, features=["chrono","tds73","with-tokio","tls"]`
- **keyring v4 는 v3 와 API 다름**: `keyring::use_native_store(false)` 로 백엔드 초기화, `Entry/Error` 는 `keyring-core` 에서
- Svelte 5 에서 props 초기값을 `$state(...)` 인자로 직접 쓰면 `state_referenced_locally` 경고 → `untrack(() => props.x)` 로 감싸기
- Tauri 종료 핸들러는 `Builder::build().run(|app, event| ...)` 패턴. RunEvent::ExitRequested 에서 `tauri::async_runtime::block_on` + timeout

## E2E 검증 (사용자 환경)
1. Docker SQL Server 띄우기
2. `npm run tauri dev`
3. 연결 프로필 생성 → 테스트 → 저장
4. 익스플로러 펼치기 — 시스템 DB 안 보여야 함
5. `SELECT * FROM sys.objects` 실행 → 그리드 표시
6. 행 선택 → ⌘C → 다른 곳에 붙여넣기 → TSV 확인
7. `WAITFOR DELAY '00:00:10'` 후 ⌘. 또는 오버레이 취소
8. 결과 → Excel 저장 → .xlsx 열어서 타입 확인
9. 앱 X 닫기 → 콘솔에 `[shutdown] ... 정리: 1건` 로그
10. ⌘T 로 탭 2개 만들고 한쪽 `WAITFOR DELAY '00:00:05'` 실행 → 즉시 다른 탭으로 전환 → 빠른 쿼리 실행 → 두 결과가 각각 표시
11. dirty 탭 ⌘W → 확인 다이얼로그 → 취소/닫기 동작
12. 마지막 탭 ⌘W → 빈 새 탭 자동 생성
13. Object Explorer 핸들 좌우 드래그 → 180/600 한계 클램프, 재시작 후 폭 유지

## 향후 확장 (V2)
- 결과 스트리밍 IPC
- 다중 결과셋 표시
- 컬럼 필터/검색 강화
- 결과 → CSV/JSON 내보내기 옵션
