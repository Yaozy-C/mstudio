export type StorageCategory = {
  bytes: number;
  files: number;
  reclaimableBytes: number;
};
export type StorageUsage = {
  totalBytes: number;
  mediaBytes: number;
  dataBytes: number;
  reclaimableBytes: number;
  unusedFiles: number;
  unusedRecords: number;
  categories: Record<string, StorageCategory>;
  items: { kind: string; name: string; bytes: number }[];
  fingerprint: string;
  canClean: boolean;
};
export type CleanupResult = {
  report: StorageUsage;
  freedBytes: number;
  deletedFiles: number;
  deletedRecords: number;
  warning: string;
};
export function formatBytes(bytes: number) {
  const value = Math.max(0, Number.isFinite(bytes) ? bytes : 0);
  if (value < 1024) return `${Math.round(value)} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  const power = Math.min(Math.floor(Math.log2(value) / 10), units.length);
  return `${(value / 1024 ** power).toFixed(1)} ${units[power - 1]}`;
}
