# 다중 쿼리 탭 & 리사이즈 가능한 Object Explorer

작성일: 2026-05-19
대상 브랜치: main
관련 컨텍스트: `context.md` (V2 항목 "다중 쿼리 탭")

## 1. 목적

워크스페이스에서 여러 쿼리를 동시에 다룰 수 있도록 **탭 UI** 를 도입하고,
좌측 **Object Explorer 의 폭을 마우스 드래그로 조절** 할 수 있게 한다.

## 2. 결정 사항 요약 (사용자 합의)

| 항목 | 결정 |
|---|---|
| 탭 실행 모델 | **탭별 독립 실행** — 백그라운드 동시 실행 허용 |
| 탭 영속성 | **복원 안 함** — 재시작 시 빈 탭 1개로 시작 |
| 익스플로러 폭 | **드래그 + localStorage** 저장 |
| 단축키 | `⌘T` 새 탭 / `⌘W` 탭 닫기 |
| 툴바 설정 | **전역** (maxRows / timeoutSec / sortable 모든 탭 공유) |
| dirty 닫기 | **확인 다이얼로그** |
| 상태 관리 | **별도 스토어 분리** (`queryTabs.svelte.ts`) |

## 3. 아키텍처

### 3.1 신규 파일

- `src/lib/stores/queryTabs.svelte.ts` — 탭 상태/조작 스토어 (Svelte 5 runes class)
- `src/lib/components/QueryTabsBar.svelte` — 탭바 (칩 리스트 + `+` 버튼)
- `src/lib/components/ObjectExplorerPane.svelte` — `ObjectExplorer` 래퍼 + 좌우 리사이즈 핸들 + localStorage

### 3.2 수정 파일

- `src/routes/workspace/+page.svelte`
  - 모든 쿼리/파일/결과 상태를 활성 탭 기준으로 재배선
  - 새 단축키 (`⌘T`, `⌘W`)
  - `aside` 제거 → `<ObjectExplorerPane>` 로 교체
  - dirty 닫기 확인 다이얼로그 추가

- `context.md` — V2 진행 상태 갱신, E2E 시나리오 추가

## 4. 데이터 모델

```ts
// $lib/stores/queryTabs.svelte.ts
import type { QueryResult, SqlClassification } from '$lib/api/query';

export interface QueryTab {
  id: string;                  // crypto.randomUUID()
  title: string;               // 파일명 or "쿼리 N"
  sql: string;
  selectedSql: string;         // QueryEditor 와 양방향 bind
  filePath: string | null;
  savedContent: string | null; // 저장 시점의 sql 스냅샷
  result: QueryResult | null;
  runError: string | null;
  busy: boolean;
  currentQueryId: string | null;
  pendingClassification: SqlClassification | null;
  exporting: boolean;
}
```

### dirty 판정

```ts
function isDirty(t: QueryTab): boolean {
  if (t.filePath !== null) return t.sql !== t.savedContent;
  return t.sql.trim() !== '';
}
```

## 5. 스토어 API

```ts
class QueryTabsStore {
  tabs = $state<QueryTab[]>([]);
  activeId = $state<string | null>(null);
  #draftCounter = 1;

  get active(): QueryTab | null;

  init(): void;                                           // 빈 탭 1개 보장
  addTab(opts?: { sql?: string; filePath?: string; title?: string }): string;
  closeTab(id: string): { needsConfirm: boolean };
  forceCloseTab(id: string): void;
  switchTo(id: string): void;

  updateTab(id: string, patch: Partial<QueryTab>): void;
  updateActive(patch: Partial<QueryTab>): void;
}
export const queryTabs = new QueryTabsStore();
```

핵심 동작:

- `addTab` 은 새 탭을 즉시 활성화. title 미지정이면 `"쿼리 ${#draftCounter++}"`.
- `closeTab(id)`: dirty 면 `{ needsConfirm: true }` 만 반환. 호출자가 `ConfirmDialog` 표시 후 `forceCloseTab` 호출.
- `forceCloseTab`:
  1. 해당 탭의 `currentQueryId` 가 있으면 `queryApi.cancel(...)` (비동기, await 안 해도 됨)
  2. `tabs` 에서 제거
  3. 활성 탭이 닫혔으면 인접 탭으로 이동 (오른쪽 우선)
  4. 결과적으로 0개가 되면 빈 탭 자동 생성
- `updateTab(id, patch)` — 실행 중 탭이 비활성화돼도 결과를 안전하게 반영
- `updateActive(patch)` — 단축경로, 활성 탭 없으면 no-op

## 6. UI 구조

```
+page.svelte
├── header (기존)
└── main
    ├── ObjectExplorerPane   ← 폭 가변, 우측 드래그 핸들
    └── section.flex-1
        ├── QueryTabsBar     ← 신규
        ├── toolbar          ← 기존 (활성 탭 props 로 재배선)
        ├── QueryEditor      ← 활성 탭 sql/selectedSql bind
        ├── row splitter     ← 기존
        └── ResultGrid       ← 활성 탭 result
```

### 6.1 QueryTabsBar 디자인

- 탭 칩: `[● 쿼리 1   ×]  [📄 foo.sql   ×]  [+]`
- 활성 탭: 하단 보더(2px) 강조 + 배경색 차이
- dirty 표시: 제목 앞 `●` (없으면 공백). 닫기 버튼은 호버 시만 표시.
- 좌클릭 = 활성화, 가운데 클릭(`onauxclick` button===1) = 닫기, 우클릭은 미지원
- 탭이 많아지면 `overflow-x-auto`, `+` 버튼은 우측 고정
- 다크모드: 기존 패턴(`bg-white dark:bg-slate-900` 등) 일관 적용

### 6.2 ObjectExplorerPane 디자인

- 폭 상태: `let width = $state(readLocal() ?? 256)`
- localStorage key: `tauri-sql:explorer-width`
- 드래그 핸들: 우측 `w-1.5 cursor-col-resize`, 호버/드래그 중 색 강조 (기존 row splitter 와 같은 톤)
- 드래그 중 클램프: `[180, 600]`
- 마우스 up 에서 `localStorage.setItem(...)` (mousemove 마다 X)
- 핸들 mousedown 시 `e.preventDefault()` 로 텍스트 선택 방지

## 7. 데이터 흐름

### 7.1 새 탭

1. `⌘T` 또는 `+` 클릭
2. `queryTabs.addTab()` → 빈 탭 생성, 활성화
3. QueryEditor 가 빈 sql 로 리렌더

### 7.2 쿼리 실행 (활성 탭 = A)

1. `onRunPressed()` 진입 시 `const tabId = queryTabs.active!.id` 캡처
2. `queryApi.classify(...)` 결과로 분류
3. write/ddl/unknown 이면 `updateTab(tabId, { pendingClassification: cls })` — 다이얼로그는 활성 탭 기준으로 표시
4. 실행 진입: `updateTab(tabId, { busy: true, result: null, currentQueryId: uuid, runError: null })`
5. await 사이 사용자가 다른 탭으로 이동해도 OK — `tabId` 로 직접 패치
6. 완료: `updateTab(tabId, { busy: false, currentQueryId: null, result, runError })`

### 7.3 탭 닫기

1. `⌘W` 또는 `×` 클릭
2. `queryTabs.closeTab(id)` 호출
3. `needsConfirm` 이면 `ConfirmDialog` 표시 → 확인 시 `forceCloseTab(id)`
4. 아니면 즉시 `forceCloseTab(id)`

### 7.4 파일 열기/저장

- 열기: 활성 탭에 로드, `filePath`, `savedContent = content`, `title = basename(path)`
  - 단, 현재 활성 탭이 빈 드래프트(`filePath===null && sql===''`) 면 그 탭을 재사용. 아니면 새 탭을 만들어 로드.
- 저장 성공: `savedContent = sql`, `title = basename(path)`

### 7.5 활성 탭 변경

- `queryTabs.switchTo(id)` 호출
- 백그라운드 쿼리는 영향 없음 (각 탭의 busy/currentQueryId 가 독립)
- 활성 탭의 `pendingClassification` 이 있으면 `ConfirmDialog` 가 다시 보임 (자연스러움)

## 8. 단축키

| 키 | 동작 | 비고 |
|---|---|---|
| ⌘T | 새 탭 | 신규 |
| ⌘W | 활성 탭 닫기 (dirty 면 확인) | 신규 |
| ⌘. | 활성 탭 쿼리 취소 | 기존 |
| ⌘O | 파일 열기 (활성 탭 규칙) | 기존 동작 유지, 단 위 7.4 규칙 적용 |
| ⌘S / ⇧⌘S | 저장 / 다른 이름으로 | 기존 |
| ⌘↵ | 실행 | 기존 (QueryEditor 내부) |

`window` 레벨 keydown 에서 처리. ⌘W 가 macOS Tauri 윈도우 닫기와 충돌하는지 확인 필요 — 충돌 시 `e.preventDefault()` 로 가로채고 탭 닫기로 한정. 마지막 탭에서 ⌘W 를 누르면 새 빈 탭이 자동 생성되므로 윈도우는 닫히지 않는다.

## 9. 마이그레이션 영향도

- 기존 동작 보존:
  - 초기 진입 시 빈 새 탭 1개가 자동 생성됨 — 기존처럼 즉시 SQL 작성/실행 가능
  - 툴바 동작/단축키 모두 활성 탭에 적용 (사용자 체감 동일)
- 파일 열기 동작이 살짝 바뀜: 빈 드래프트 탭이면 그 자리에, 아니면 새 탭으로 — 이전엔 항상 같은 자리였음. 합리적인 변화.

## 10. 보안 / 안정성

- 새로운 IPC 추가 없음, Rust 변경 없음
- localStorage 사용은 단순 숫자(폭). XSS 표면 추가 없음
- 백그라운드 실행 탭이 닫힐 때 `queryApi.cancel` 으로 정리 — 좀비 쿼리 방지
- 앱 종료 시 기존 `RunEvent::ExitRequested` → `cancel_all` 동작이 모든 탭의 쿼리도 정리함

## 11. 테스트 계획

본 프로젝트엔 단위 테스트 인프라가 없으므로 **수동 E2E 시나리오** 로 검증. `context.md` E2E 섹션에 다음을 추가:

1. ⌘T 로 탭 2개 생성 → 각 탭에 다른 SQL → 한쪽에서 `WAITFOR DELAY '00:00:05'` 실행 → 즉시 다른 탭으로 전환 → 그 탭에서 빠른 쿼리 실행 → 두 결과가 각각의 탭에 표시되는지
2. dirty 탭 ⌘W → 확인 다이얼로그 → 취소 누르면 유지, 확인 누르면 닫힘
3. 마지막 탭 ⌘W → 빈 새 탭이 자동 생성되는지
4. Object Explorer 핸들 드래그 → 폭이 부드럽게 변하는지, 180/600 한계 클램프되는지
5. 앱 재시작 → 익스플로러 폭이 직전 값으로 복원되는지
6. 백그라운드 실행 중인 탭을 ⌘W 로 닫으면 콘솔에 cancel 호출이 나가는지 (또는 Rust 로그)

## 12. 비범위 (Out of Scope)

- 탭 드래그 재정렬
- 탭 영속화 (재시작 복원)
- 탭별 결과 히스토리/이전 결과 보관
- 윈도우 분할 (split pane)

향후 V2 에서 별도 spec 으로 다룬다.

## 13. 구현 순서 (러프)

1. `queryTabs.svelte.ts` 스토어 + dirty 헬퍼
2. `QueryTabsBar.svelte`
3. `ObjectExplorerPane.svelte`
4. `+page.svelte` 리팩터: 활성 탭 기반 wiring, 단축키, 닫기 다이얼로그
5. context.md 갱신

writing-plans 스킬이 위 순서를 단계별 실행 계획으로 풀어준다.
