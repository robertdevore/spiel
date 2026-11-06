import type { ModelView } from "./contracts";
import { fmtBytes, formatTimeAgo } from "./ui";

export type Downloads = Record<string, { downloaded: number; total: number | null }>;
interface ModelActions {
  useModel: (modelId: string) => void;
  deleteModel: (modelId: string) => void;
  download: (modelId: string) => void;
}

export function modelsCard(models: ModelView[], downloads: Downloads, actions: ModelActions): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Speech model</p>`;

  for (const m of models) {
    const row = document.createElement("div");
    row.className = `model${m.is_current ? " current" : ""}`;
    const meta = document.createElement("div");
    meta.className = "meta";
    const sizeText = m.install_bytes ? `${fmtBytes(m.install_bytes)}` : "not present";
    const statusParts: string[] = [
      m.install_status === "installed" ? "Installed" : `${m.install_status} (${sizeText})`,
    ];
    if (!m.installed && m.install_reason) {
      statusParts.push(m.install_reason);
    }
    if (!m.installed && m.install_modified_ms !== null) {
      statusParts.push(`modified ${formatTimeAgo(m.install_modified_ms)}`);
    }

    const statusText = statusParts.join(" · ");
    const badgeClass = m.install_status === "installed" ? "badge" : "warn";
    meta.innerHTML = `<span class="name">${m.label} · ~${m.approx_mb} MB</span>
      <span class="note">${m.note}</span>
      <span class="note ${badgeClass}">${statusText}</span>`;
    row.appendChild(meta);

    const dl = downloads[m.id];
    if (dl) {
      const pct = dl.total ? Math.round((dl.downloaded / dl.total) * 100) : 0;
      const wrap = document.createElement("div");
      wrap.style.flex = "1";
      wrap.innerHTML = `<div class="progress"><div style="width:${pct}%"></div></div>
        <div class="note">${fmtBytes(dl.downloaded)}${dl.total ? " / " + fmtBytes(dl.total) : ""}</div>`;
      row.appendChild(wrap);
    } else if (m.installed) {
      if (m.is_current) {
        const badge = document.createElement("span");
        badge.className = "badge";
        badge.textContent = "Active ✓";
        row.appendChild(badge);
      } else {
        const useBtn = document.createElement("button");
        useBtn.textContent = "Use";
        useBtn.onclick = () => actions.useModel(m.id);
        row.appendChild(useBtn);

        const deleteBtn = document.createElement("button");
        deleteBtn.className = "danger";
        deleteBtn.textContent = "Delete";
        deleteBtn.onclick = () => {
          if (confirm(`Delete model ${m.label}? This frees disk and unloads any cache.`)) {
            actions.deleteModel(m.id);
          }
        };
        row.appendChild(deleteBtn);
      }
    } else {
      const isRecoverable = m.install_status === "partial" || m.install_status === "corrupt";
      const dlBtn = document.createElement("button");
      dlBtn.textContent = isRecoverable ? "Repair" : "Download";
      dlBtn.onclick = () => actions.download(m.id);
      row.appendChild(dlBtn);
    }
    card.appendChild(row);
  }
  return card;
}
