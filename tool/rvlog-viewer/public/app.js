document.body.addEventListener("htmx:afterSwap", (ev) => {
  const detail = ev.detail;
  const target = detail?.target;
  if (!target || target.id !== "disasmPanel") return;

  const wrap = target.querySelector(".disasmWrap");
  const firstId = wrap?.dataset.firstHl;
  if (!firstId) return;

  const el = document.getElementById(firstId);
  el?.scrollIntoView({ block: "center" });
});

document.body.addEventListener("click", (ev) => {
  const btn = ev.target?.closest?.(".logLine");
  if (!btn) return;

  document.querySelectorAll(".logLine.selected").forEach((x) => x.classList.remove("selected"));
  btn.classList.add("selected");
});
