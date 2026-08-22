const KB = 1024;
const MB = KB * 1024;
const GB = MB * 1024;

/** Human-readable disk size for recorded meeting audio chips. */
export function formatAudioBytes(bytes: number): string {
  const n = Number.isFinite(bytes) ? Math.max(0, bytes) : 0;
  if (n === 0) return "0 MB";
  if (n < MB) {
    const kb = n / KB;
    return `${kb < 10 ? kb.toFixed(1) : Math.round(kb)} KB`;
  }
  if (n < GB) {
    const mb = n / MB;
    return `${mb < 10 ? mb.toFixed(1) : Math.round(mb)} MB`;
  }
  return `${(n / GB).toFixed(1)} GB`;
}
