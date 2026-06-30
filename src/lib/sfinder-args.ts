import type { SfinderCommandConfig } from '@/types/sfinder';

/**
 * Build CLI argument array from SfinderCommandConfig.
 * Used for display/debug only — actual args built in Rust.
 */
export function buildCliArgs(config: SfinderCommandConfig): string[] {
  const args: string[] = [];

  args.push(config.command);

  for (const t of config.tetfu) {
    if (t) args.push('--tetfu', t);
  }
  if (config.page !== undefined) {
    args.push('--page', String(config.page));
  }
  if (config.clearLine !== undefined) {
    args.push('--clear-line', String(config.clearLine));
  }
  if (config.patterns) {
    args.push('--patterns', config.patterns);
  }
  if (config.hold) {
    args.push('--hold', config.hold);
  }
  if (config.drop) {
    args.push('--drop', config.drop);
  }
  if (config.kicks) {
    args.push('--kicks', config.kicks);
  }
  if (config.format) {
    args.push('--format', config.format);
  }
  if (config.split) {
    args.push('--split', 'yes');
  }
  if (config.specifiedOnly) {
    args.push('--specified-only');
  }
  if (config.reserved) {
    args.push('--reserved');
  }
  if (config.outputBase) {
    args.push('--output-base', config.outputBase);
  }
  if (config.maxLayer) {
    args.push('--max-layer', String(config.maxLayer));
  }
  if (config.key) {
    args.push('--key', config.key);
  }
  if (config.mode) {
    args.push('--mode', config.mode);
  }
  return args;
}

/**
 * Format CLI args for display.
 */
export function formatCommandLine(config: SfinderCommandConfig, jarPath: string, javaPath?: string): string {
  const java = javaPath || 'java';
  const args = buildCliArgs(config);
  return `${java} -jar ${jarPath} ${args.join(' ')}`;
}
