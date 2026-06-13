export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 10 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatImageDimensions(
  width: number | null | undefined,
  height: number | null | undefined,
): string | null {
  if (width != null && height != null && width > 0 && height > 0) {
    return `${width.toLocaleString()} × ${height.toLocaleString()}`;
  }
  return null;
}

export function formatImageMeta(
  width: number | null | undefined,
  height: number | null | undefined,
  byteSize: number | null | undefined,
): string {
  const parts: string[] = [];
  const dimensions = formatImageDimensions(width, height);
  if (dimensions) parts.push(dimensions);
  if (byteSize != null && byteSize > 0) {
    parts.push(formatBytes(byteSize));
  }
  return parts.length > 0 ? parts.join(" · ") : "—";
}
