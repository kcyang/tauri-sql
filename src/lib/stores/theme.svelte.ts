// 라이트/다크/시스템 테마 store.
// - 'system' 일 때는 `prefers-color-scheme` 미디어 쿼리를 따른다.
// - 선택값은 localStorage('theme') 에 저장.
// - 적용은 documentElement 의 'dark' 클래스로 한다.

export type ThemeChoice = "system" | "light" | "dark";

const STORAGE_KEY = "theme";

function readStored(): ThemeChoice {
  if (typeof localStorage === "undefined") return "system";
  const v = localStorage.getItem(STORAGE_KEY);
  if (v === "light" || v === "dark" || v === "system") return v;
  return "system";
}

function systemPrefersDark(): boolean {
  if (typeof window === "undefined") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function computeEffective(choice: ThemeChoice): "light" | "dark" {
  if (choice === "system") return systemPrefersDark() ? "dark" : "light";
  return choice;
}

let choice = $state<ThemeChoice>("system");
let effective = $state<"light" | "dark">("light");

function applyEffective() {
  effective = computeEffective(choice);
  if (typeof document === "undefined") return;
  if (effective === "dark") {
    document.documentElement.classList.add("dark");
  } else {
    document.documentElement.classList.remove("dark");
  }
}

let initialized = false;

export const themeStore = {
  get choice() {
    return choice;
  },
  get effective() {
    return effective;
  },
  get isDark() {
    return effective === "dark";
  },
  init() {
    if (initialized) return;
    initialized = true;
    choice = readStored();
    applyEffective();
    // 시스템 변경 추적 (choice 가 'system' 일 때만 의미 있음)
    if (typeof window !== "undefined") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      mq.addEventListener("change", () => {
        if (choice === "system") applyEffective();
      });
    }
  },
  set(next: ThemeChoice) {
    choice = next;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(STORAGE_KEY, next);
    }
    applyEffective();
  },
};
