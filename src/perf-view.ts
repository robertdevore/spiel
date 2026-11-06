import type { PerfSnapshot, MemorySnapshot } from "./contracts";
import { fmtBytes } from "./ui";

export function perfCard(p: PerfSnapshot, m: MemorySnapshot | null, onClear: () => void): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `<p class="section-title">Performance Profile</p>`;

  if (m) {
    const rss = document.createElement("div");
    rss.className = "privacy";
    rss.textContent = `Current app RSS: ${fmtBytes(m.rss_bytes)} (${m.source})`;
    card.appendChild(rss);
  }

  const summary = document.createElement("div");
  summary.className = "privacy";
  summary.textContent =
    `Samples: ${p.sample_count} · Avg: ${p.average_total_ms}ms · P50: ${p.p50_total_ms}ms · ` +
    `P95: ${p.p95_total_ms}ms · Max: ${p.max_total_ms}ms · Over budget (${p.budget_ms}ms): ${p.over_budget_count}`;
  card.appendChild(summary);

  const stageSummary = document.createElement("div");
  stageSummary.className = "privacy";
  stageSummary.textContent =
    `Stage averages: capture ${p.average_capture_ms}ms · transcribe ${p.average_transcribe_ms}ms · insert ${p.average_insert_ms}ms`;
  card.appendChild(stageSummary);

  const outcomeSummary = document.createElement("div");
  outcomeSummary.className = "privacy";
  outcomeSummary.textContent =
    `Outcomes: pasted ${p.pasted_count} · clipboard-only ${p.clipboard_only_count} · insert errors ${p.insert_error_count}`;
  card.appendChild(outcomeSummary);

  if (p.last) {
    const last = document.createElement("div");
    last.className = "privacy";
    last.textContent =
      `Last: total ${p.last.total_ms}ms (capture ${p.last.capture_ms}ms, transcribe ${p.last.transcribe_ms}ms, ` +
      `insert ${p.last.insert_ms}ms) · chars ${p.last.text_chars} · outcome ${p.last.outcome}`;
    card.appendChild(last);
  }

  const downloadSummary = document.createElement("div");
  downloadSummary.className = "privacy";
  downloadSummary.textContent =
    `Download samples: ${p.download_sample_count} · Avg ${p.average_download_ms}ms · ` +
    `P95 ${p.p95_download_ms}ms · Max ${p.max_download_ms}ms`;
  card.appendChild(downloadSummary);

  if (p.last_download) {
    const lastDownload = document.createElement("div");
    lastDownload.className = "privacy";
    lastDownload.textContent =
      `Last download: ${p.last_download.total_ms}ms · ${fmtBytes(p.last_download.downloaded_bytes)} ` +
      `${p.last_download.expected_bytes ? `/ ${fmtBytes(p.last_download.expected_bytes)}` : ""} · ` +
      `outcome ${p.last_download.outcome}`;
    card.appendChild(lastDownload);
  }

  const controls = document.createElement("div");
  controls.className = "row";
  const clearBtn = document.createElement("button");
  clearBtn.textContent = "Clear Profile Samples";
  clearBtn.onclick = onClear;
  controls.appendChild(clearBtn);
  card.appendChild(controls);

  return card;
}
