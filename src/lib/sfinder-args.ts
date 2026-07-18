import type { SfinderCommandConfig } from '@/types/sfinder';

/**
 * Build CLI argument array from SfinderCommandConfig.
 * Used for display/debug only — actual args built in Rust.
 */
export function buildCliArgs(config: SfinderCommandConfig): string[] {
  const args: string[] = [];
  const is_cover = config.command === 'cover';
  const is_spin = config.command === 'spin';

  args.push(config.command);

  for (const t of config.tetfu) {
    if (t) args.push('--tetfu', t);
  }
  if (!is_cover && !is_spin && config.page !== undefined) {
    args.push('--page', String(config.page));
  }
  if (is_spin) {
    if (config.line !== undefined) args.push('--line', String(config.line));
  } else if (config.clearLine !== undefined) {
    args.push(is_cover ? '--max-clearline' : '--clear-line', String(config.clearLine));
  }
  if (config.patterns) {
    args.push('--patterns', config.patterns);
  }
  if (!is_spin && config.hold) {
    args.push('--hold', config.hold);
  }
  if (!is_spin && config.drop) {
    args.push('--drop', config.drop);
  }
  if (!is_spin && config.kicks) {
    args.push('--kicks', config.kicks);
  }
  if (!is_cover && !is_spin && config.format) {
    args.push('--format', config.format);
  }
  if (config.split) {
    args.push('--split', 'yes');
  }
  if (!is_cover && config.specifiedOnly) {
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
  if (!is_cover && !is_spin && config.key) {
    args.push('--key', config.key);
  }
  if (is_cover && config.mode) {
    args.push('--mode', config.mode);
  }
  // spin-only
  if (is_spin) {
    if (config.fillBottom !== undefined) args.push('--fill-bottom', String(config.fillBottom));
    if (config.fillTop !== undefined) args.push('--fill-top', String(config.fillTop));
    if (config.marginHeight !== undefined) args.push('--margin-height', String(config.marginHeight));
    if (config.roof !== undefined) args.push('--roof', config.roof ? 'yes' : 'no');
    if (config.maxRoof !== undefined) args.push('--max-roof', String(config.maxRoof));
    if (config.filter) args.push('--filter', config.filter);
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
