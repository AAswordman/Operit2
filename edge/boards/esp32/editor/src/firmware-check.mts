/** Results from the actual Xtensa build and espflash image generator. */
export interface FirmwareCheck {
  status: 'not-checked' | 'checking' | 'passed' | 'failed';
  profile?: 'standalone-apps' | 'remote-display';
  stage?: 'compile' | 'abi' | 'partition';
  checkedAt?: string;
  compilePassed?: boolean;
  abiPassed?: boolean;
  partitionPassed?: boolean;
  partitionBytes?: number;
  imageBytes?: number;
  overflowBytes?: number;
  elfSha256?: string;
  error?: string;
}

export const firmwareReportPrefix = 'OPERIT_FIRMWARE_CHECK=';

/** Read the actual factory app capacity, accepting ESP-IDF hex and K/M sizes. */
export function appPartitionBytes(csv: string): number {
  const entries = csv.split(/\r?\n/).map(line => line.split('#')[0].split(',').map(field => field.trim()));
  const apps = entries.filter(fields => ['app', '0', '0x00'].includes(fields[1]) && fields.length >= 5);
  const app = apps.find(fields => ['factory', '0', '0x00'].includes(fields[2]));
  if (!app) throw new Error('partitions.csv 缺少 factory 应用分区');
  const match = /^(0x[0-9a-f]+|[0-9]+)([km]?)$/i.exec(app[4]);
  const bytes = match ? Number(match[1]) * ({k: 1024, m: 1024 * 1024}[match[2].toLowerCase()] ?? 1) : 0;
  if (!Number.isSafeInteger(bytes) || bytes <= 0) throw new Error('factory 应用分区容量无效');
  return bytes;
}

/** Never accept an old file, a missing report, or a successful report from a failed process. */
export function firmwareCheckFromOutput(output: string, exitCode: number | null): FirmwareCheck {
  const line = output.split(/\r?\n/).reverse().find(value => value.startsWith(firmwareReportPrefix));
  let report: FirmwareCheck | undefined;
  try { report = line ? JSON.parse(line.slice(firmwareReportPrefix.length)) as FirmwareCheck : undefined; }
  catch { /* A truncated report is a failed check. */ }
  if (report?.status === 'failed' && typeof report.error === 'string') return report;
  if (exitCode === 0 && report?.status === 'passed' && report.compilePassed === true && report.abiPassed === true
      && report.partitionPassed === true && typeof report.elfSha256 === 'string' && /^[a-f0-9]{64}$/.test(report.elfSha256)
      && Number.isSafeInteger(report.imageBytes) && Number.isSafeInteger(report.partitionBytes)
      && report.imageBytes! > 0 && report.imageBytes! <= report.partitionBytes! && report.overflowBytes === 0) return report;
  return {status: 'failed', error: '固件检查未成功完成；请查看构建日志，不能使用旧构建结果。'};
}
