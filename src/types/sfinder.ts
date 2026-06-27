// --- Command enum ---
export type SfinderCommand =
  | 'percent'
  | 'path'
  | 'setup'
  | 'ren'
  | 'spin'
  | 'cover';

// --- Drop types ---
export type DropType =
  | 'softdrop'
  | 'harddrop'
  | '180'
  | 't-softdrop';

// --- Hold options ---
export type HoldOption = 'use' | 'avoid';

// --- Output format ---
export type OutputFormat = 'html' | 'csv';

// --- CSV sort key ---
export type CsvKey = 'solution' | 'pattern' | 'use';

// --- Unified command configuration ---
export interface SfinderCommandConfig {
  command: SfinderCommand;
  tetfu: string;
  jarPath?: string;
  javaPath?: string;
  page?: number;
  clearLine?: number;
  patterns?: string;
  hold?: HoldOption;
  drop?: DropType;
  kicks?: string;
  format?: OutputFormat;
  split?: boolean;
  specifiedOnly?: boolean;
  reserved?: boolean;
  outputBase?: string;
  fieldPath?: string;
  patternsPath?: string;
  threads?: number;
  maxLayer?: number;
  key?: CsvKey;
}

// --- Output from Rust backend ---
export interface PathResultEntry {
  fumen: string;
  coverage: number;
  used: string;
}

export interface SfinderOutput {
  stdout: string;
  stderr: string;
  exitCode: number;
  outputFiles: string[];
  commandLine: string;
  pathResults?: PathResultEntry[];
  pathTotalPatterns?: number;
  strictMinimal?: PathResultEntry[];
}

// --- Command execution state ---
export type CommandStatus =
  | { type: 'idle' }
  | { type: 'running'; startTime: number }
  | { type: 'success'; output: SfinderOutput }
  | { type: 'error'; message: string; stderr?: string }
  | { type: 'cancelled' };

// --- Recent command history entry ---
export interface CommandHistoryEntry {
  id: string;
  config: SfinderCommandConfig;
  output: SfinderOutput;
  timestamp: number;
}
