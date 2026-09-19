/** Renders a byte count for the mono lane. Binary units, because that is what
 *  a file manager reports, and one decimal above the kilobyte so a slider
 *  nudge visibly moves the number. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${Math.round(bytes)} B`;

  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  return `${(kb / 1024).toFixed(1)} MB`;
}
