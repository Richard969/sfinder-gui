/**
 * Parse sfinder CLI output into structured data for display.
 */

import { decoder } from 'tetris-fumen';

export interface PercentResult {
  percentage: number;
  numerator?: number;
  denominator?: number;
}

export interface SolutionEntry {
  index: number;
  operations: string;
  percentage?: number;
  count?: number;
  fumen?: string;
}

export interface CoverResult {
  overallRatio: number;
  numerator?: number;
  denominator?: number;
  or?: { pct: string; fraction: string };
  and?: { pct: string; fraction: string };
}

export function parsePercent(stdout: string): PercentResult | null {
  const match = stdout.match(/(\d+\.?\d*)\s*%\s*\[(\d+)\/(\d+)\]/);
  if (match) {
    return {
      percentage: parseFloat(match[1]),
      numerator: parseInt(match[2]),
      denominator: parseInt(match[3]),
    };
  }
  return null;
}

export function parseSolutions(stdout: string): SolutionEntry[] {
  const results: SolutionEntry[] = [];
  const lines = stdout.split('\n');
  let idx = 0;
  for (const line of lines) {
    const match = line.match(/([\w\-\s]+)\s*\/\s*(\d+\.?\d*)\s*%\s*\[(\d+)\]/);
    if (match) {
      results.push({
        index: idx++,
        operations: match[1].trim(),
        percentage: parseFloat(match[2]),
        count: parseInt(match[3]),
      });
    }
  }
  return results;
}

export function parseCoverage(stdout: string): CoverResult | null {
  const match = stdout.match(/OR\s*=\s*(\d+\.?\d*)\s*%\s*\[(\d+)\/(\d+)\]/i);
  if (match) {
    return {
      overallRatio: parseFloat(match[1]),
      numerator: parseInt(match[2]),
      denominator: parseInt(match[3]),
    };
  }
  return null;
}

export function parseKickVerification(stdout: string): { kick: string; valid: boolean }[] {
  const results: { kick: string; valid: boolean }[] = [];
  const lines = stdout.split('\n');
  for (const line of lines) {
    const okMatch = line.match(/(.+?):\s*(OK|PASS)/i);
    const failMatch = line.match(/(.+?):\s*(FAIL|ERROR)/i);
    if (okMatch) {
      results.push({ kick: okMatch[1].trim(), valid: true });
    } else if (failMatch) {
      results.push({ kick: failMatch[1].trim(), valid: false });
    }
  }
  return results;
}

export function parseCsv(csvText: string): { headers: string[]; rows: string[][] } {
  const lines = csvText.trim().split('\n');
  if (lines.length === 0) return { headers: [], rows: [] };
  const headers = lines[0].split(',').map((h) => h.trim());
  const rows = lines.slice(1).map((line) => line.split(',').map((c) => c.trim()));
  return { headers, rows };
}

export interface SpinEntry {
  index: number;
  operations: string;
  mark: 'O' | 'X' | '-';
  fumen?: string;
  clear: number;
  hole: number;
  piece: number;
  /** T-spin category: 'single-regular' | 'single-mini' | 'double-regular' | 'double-mini' | 'triple-regular' | '' */
  category: string;
}

export function parseSpin(html: string): SpinEntry[] {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  const results: SpinEntry[] = [];
  let idx = 0;

  // Find all sections (each section is a T-spin category)
  const sections = doc.querySelectorAll('section');
  for (const section of sections) {
    const category = section.id || '';
    const divs = section.querySelectorAll('div');
    for (const div of divs) {
      const link = div.querySelector('a[href^="http"]');
      const fumen = link?.getAttribute('href') ?? '';
      const line = div.textContent?.trim() ?? '';
      const statsMatch = line.match(/clear=(\d+),\s*hole=(\d+),\s*piece=(\d+)/);
      if (!statsMatch) continue;
      const markMatch = line.match(/^\[([OX\-])\]/);
      const mark = (markMatch ? markMatch[1] : '-') as 'O' | 'X' | '-';
      const ops = line.replace(/^\[[OX\-]\]\s*/, '').replace(/\[clear=.*?\]\s*/, '').trim();
      results.push({
        index: idx++, operations: ops, mark,
        fumen: fumen.replace('http://fumen.zui.jp/?', ''),
        clear: parseInt(statsMatch[1]), hole: parseInt(statsMatch[2]),
        piece: parseInt(statsMatch[3]), category,
      });
    }
  }

  if (results.length > 0) return results;

  // Fallback: plain-text lines
  for (const line of html.split('\n')) {
    const statsMatch = line.match(/clear=(\d+),\s*hole=(\d+),\s*piece=(\d+)/);
    if (!statsMatch) continue;
    const markMatch = line.match(/^\[([OX\-])\]/);
    const mark = (markMatch ? markMatch[1] : '-') as 'O' | 'X' | '-';
    const ops = line.replace(/^\[[OX\-]\]\s*/, '').replace(/\[clear=.*?\]\s*/, '').trim();
    results.push({ index: idx++, operations: ops, mark, fumen: '',
      clear: parseInt(statsMatch[1]), hole: parseInt(statsMatch[2]),
      piece: parseInt(statsMatch[3]), category: '' });
  }
  return results;
}
