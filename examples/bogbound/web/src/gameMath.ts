export function formatTime(ms: number) {
  const seconds = Math.ceil(Math.max(0, ms) / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
export function clampAxis(value: number) {
  return Math.max(-1, Math.min(1, Number.isFinite(value) ? value : 0));
}
