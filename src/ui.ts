export function fmtBytes(n: number): string {
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatTimeAgo(modifiedMs: number): string {
  const ageMs = Date.now() - modifiedMs;
  if (ageMs < 60_000) return "just now";
  const ageMins = Math.floor(ageMs / 60_000);
  if (ageMins < 60) return `${ageMins} min ago`;
  const ageHours = Math.floor(ageMins / 60);
  if (ageHours < 24) return `${ageHours} hr ago`;
  const ageDays = Math.floor(ageHours / 24);
  return `${ageDays}d ago`;
}

export function textField(
  label: string,
  value: string,
  onCommit: (v: string) => void,
  placeholder = "",
): HTMLElement {
  const wrap = document.createElement("label");
  wrap.className = "field";
  wrap.textContent = label;
  const input = document.createElement("input");
  input.type = "text";
  input.value = value;
  input.placeholder = placeholder;
  const commit = () => {
    if (input.value.trim() && input.value !== value) onCommit(input.value.trim());
  };
  input.onblur = commit;
  input.onkeydown = (e) => {
    if (e.key === "Enter") input.blur();
  };
  wrap.appendChild(input);
  return wrap;
}

export function numberField(
  label: string,
  value: number,
  min: number,
  max: number,
  onCommit: (v: number) => void,
): HTMLElement {
  const wrap = document.createElement("label");
  wrap.className = "field";
  wrap.textContent = label;
  const input = document.createElement("input");
  input.type = "number";
  input.min = String(min);
  input.max = String(max);
  input.value = String(value);
  input.onblur = () => {
    const raw = Number(input.value);
    const normalized = Number.isFinite(raw) ? Math.round(raw) : value;
    const v = Math.max(min, Math.min(max, normalized));
    input.value = String(v);
    if (v !== value) onCommit(v);
  };
  wrap.appendChild(input);
  return wrap;
}

export function toggleField(label: string, value: boolean, onChange: (v: boolean) => void): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = "toggle";
  wrap.textContent = label;
  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = value;
  input.onchange = () => onChange(input.checked);
  wrap.appendChild(input);
  return wrap;
}

export function escapeHtml(s: string): string {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}
