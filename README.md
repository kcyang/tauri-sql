# tauri-sql

macOS 에서 가볍게 쓰는 MSSQL 쿼리/탐색 데스크톱 도구.
Tauri 2 + SvelteKit (Svelte 5) + Rust(tiberius) 기반.

## 주요 기능

### 연결 관리
- 다중 연결 프로필 저장 (`~/Library/Application Support/com.kcyang.tauri-sql/profiles.json`)
- **비밀번호는 macOS Keychain** 에 분리 저장 (keyring v4)
- 연결 테스트 / 프로필 편집 / 삭제

### 워크스페이스
- **오브젝트 익스플로러** (좌측): DB / 테이블 / 뷰 / 저장 프로시저를 lazy 로드로 탐색
  - 시스템 DB (`master`/`msdb`/`tempdb`) 는 자동 숨김
  - 객체 더블클릭 → 새 쿼리 탭에 `SELECT TOP 100 * FROM ...` 또는 `EXEC ...` 템플릿 삽입
- **SQL 에디터** (CodeMirror 6 + lang-sql)
  - 키워드 색상 강조 (라이트/다크 각각 8색 팔레트)
  - 줄번호, 자동 들여쓰기, 괄호 매칭, undo/redo
  - **선택 영역만 실행** — 일부 텍스트만 선택해서 ⌘↵ 또는 "선택 실행" 버튼
- **결과 그리드** (AG Grid Community)
  - 가상 스크롤 (수만 행 안정 렌더)
  - 컬럼 폭은 데이터 기반 자동 사이징 (최대 600px 컷)
  - 행 선택(클릭/⌘클릭/⇧클릭) + ⌘C 로 TSV(헤더 포함) 클립보드 복사
  - 셀 안 텍스트 일부만 드래그 선택해서 복사도 가능
  - 정렬은 기본 OFF (툴바 "정렬 허용" 체크박스로 켜기)
- **실행 중 오버레이**: 결과 영역에 회전 스피너 + 경과 시간 + 취소 버튼

### 안정성 장치
- 자동 max_rows 컷 (기본 1,000 — 100 / 1,000 / 10,000 / 100,000 선택)
- 쿼리 취소 (cancel 토큰 + connection invalidate)
- 쿼리 타임아웃 (기본 30초 — 10초 / 30초 / 1분 / 5분 / 1시간)
- 쓰기/DDL 쿼리는 실행 전 **확인 다이얼로그** (SQL 분류기로 INSERT/UPDATE/DELETE/CREATE/ALTER/DROP/EXEC 감지)
- bb8 풀 `test_on_check_out` 으로 취소된 쿼리의 손상 가능성 있는 커넥션 자동 폐기
- 앱 종료 시 graceful cleanup: 활성 쿼리 cancel → 모든 세션 close → TCP socket close (3초 타임아웃)

### 입출력
- **파일 열기/저장** (`.sql`): ⌘O / ⌘S, ⇧⌘S = 다른 이름으로
- **Excel 내보내기** (`.xlsx`): 타입 보존 (숫자/불리언/날짜는 네이티브, decimal/uuid 등은 문자열). 헤더 굵게 + autofit

### 모양
- **다크모드**: 라이트 / 다크 / 시스템 3-way 토글 (localStorage 유지)
- 에디터/그리드 사이 **드래그 핸들** 로 높이 조절

## 단축키

| 키 | 동작 |
|---|---|
| ⌘↵ | 쿼리 실행 (선택 영역 있으면 그 부분만) |
| ⌘. | 진행 중인 쿼리 취소 |
| ⌘O | SQL 파일 열기 |
| ⌘S | 현재 파일에 저장 (없으면 다른 이름으로) |
| ⇧⌘S | 다른 이름으로 저장 |
| ⌘C | 그리드 행/셀 클립보드 복사 (포커스가 그리드 안일 때만) |

## 개발 환경

요구사항:
- Node.js 20+, npm
- Rust toolchain (1.83+ 권장)
- macOS: Xcode CLT (`xcode-select --install`)

```bash
# 의존성 설치
npm install

# 개발 모드 (Vite + Tauri 동시 실행)
npm run tauri dev

# 프로덕션 빌드 (.app + .dmg)
npm run tauri build

# 단위 테스트 (Rust)
cd src-tauri && cargo test --lib

# 타입 체크
npm run check
```

## 테스트용 로컬 MSSQL

```bash
docker run -e "ACCEPT_EULA=Y" -e "MSSQL_SA_PASSWORD=Test_1234!" \
  -p 1433:1433 --name mssql-dev \
  -d mcr.microsoft.com/azure-sql-edge:latest

# 또는 x86_64 환경
docker run -e "ACCEPT_EULA=Y" -e "MSSQL_SA_PASSWORD=Test_1234!" \
  -p 1433:1433 --name mssql-dev \
  -d mcr.microsoft.com/mssql/server:2022-latest
```

연결 프로필:
- 호스트: `localhost`
- 포트: `1433`
- DB: `master`
- 사용자: `sa`
- 비밀번호: `Test_1234!`
- 서버 인증서 신뢰: ON (자체 서명)

## 아키텍처

```
src-tauri/                Rust 백엔드 (~1,400 줄)
  src/
    lib.rs                Tauri Builder + RunEvent (종료 cleanup)
    error.rs              AppError (serde 직렬화 가능)
    types.rs              Profile / ProfileInput 등 DTO
    profiles.rs           프로필 JSON + keyring CRUD
    connection.rs         tiberius Config 빌더 + 일회성 테스트
    pool.rs               bb8-tiberius 풀 (size 4, test_on_check_out)
    session.rs            SessionManager (Uuid → Session)
    query.rs              execute_query / cancel / classify, RowValue enum
    explorer.rs           sys 카탈로그 조회 (DB/Tables/Views/Procs)
    sql_safety.rs         SQL 분류기 (read-only / write / DDL / unknown)
    file_io.rs            텍스트 read/write 커맨드
    export.rs             rust_xlsxwriter 로 .xlsx 작성
    commands.rs           Tauri 커맨드 등록

src/                      Svelte 5 + TS 프론트엔드
  routes/
    +layout.svelte        테마 적용
    +page.svelte          연결 관리 (ConnectionList)
    workspace/+page.svelte  메인 워크스페이스
  lib/
    api/                  Tauri invoke 타입드 래퍼
    stores/               session / theme (Svelte 5 runes)
    components/
      ConnectionList.svelte
      ConnectionForm.svelte
      ObjectExplorer.svelte
      QueryEditor.svelte    CodeMirror 6 호스트
      ResultGrid.svelte     AG Grid 래퍼
      QueryRunningOverlay.svelte
      ConfirmDialog.svelte
      ThemeToggle.svelte
```

## 한계 사항

- macOS 위주 — Windows 통합 인증(AD)은 미지원 (SQL Server 인증만)
- tiberius 가 네이티브 쿼리 cancel 을 지원하지 않아 "취소 = 커넥션 무효화" 패턴 사용. pool 의 `test_on_check_out` 으로 자동 복구
- 결과 셋은 첫 번째만 표시 (다중 결과셋은 향후 작업)
- 다중 쿼리 탭 미지원 (향후 작업)

## 라이선스

MIT
