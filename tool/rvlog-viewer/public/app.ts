// htmx で disasmPanel が入れ替わった後、最初のハイライト行へスクロールする
document.body.addEventListener("htmx:afterSwap", (ev: Event) => {
  const detail = (ev as CustomEvent).detail;
  const target = detail?.target as HTMLElement | undefined;
  if (!target || target.id !== "disasmPanel") return;

  const wrap = target.querySelector<HTMLElement>(".disasmWrap");
  const firstId = wrap?.dataset.firstHl;
  if (!firstId) return;

  const el = document.getElementById(firstId);
  el?.scrollIntoView({ block: "center", behavior: "instant" as ScrollBehavior });
});

// 左ログの「選択中」見た目（任意）
document.body.addEventListener("click", (ev) => {
  const btn = (ev.target as HTMLElement)?.closest?.(".logLine") as HTMLButtonElement | null;
  if (!btn) return;

  document.querySelectorAll(".logLine").forEach((x) => x.classList.remove("selected"));
  btn.classList.add("selected");
});
