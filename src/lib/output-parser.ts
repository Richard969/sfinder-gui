/**
 * Parse sfinder CLI output into structured data for display.
 */

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
}

/**
 * Extract percentage from stdout (percent command).
 * Matches patterns like:
 *   "84.64% (711/840)"
 *   "Success: 84.64%"
 */
export function parsePercent(stdout: string): PercentResult | null {
  const patterns = [
    /(\d+\.?\d*)\s*%\s*\(\s*(\d+)\s*\/\s*(\d+)\s*\)/i,
    /(?:success|rate)\s*[:=]\s*(\d+\.?\d*)\s*%/i,
    /(\d+\.?\d*)\s*%/i,
  ];

  for (const pattern of patterns) {
    const match = stdout.match(pattern);
    if (match) {
      return {
        percentage: parseFloat(match[1]),
        numerator: match[2] ? parseInt(match[2]) : undefined,
        denominator: match[3] ? parseInt(match[3]) : undefined,
      };
    }
  }
  return null;
}

/**
 * Extract solutions from stdout (path/setup/ren/spin/cover commands).
 * Matches: "OP1 OP2 OP3 / XX.X % [NNN]"
 */
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

/**
 * Extract overall ratio from coverage output.
 * Matches: "OR = 72.46 % [3652/5040]"
 */
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

/**
 * Parse verify kicks output.
 * Returns array of { kick: string, valid: boolean } entries.
 */
export function parseKickVerification(stdout: string): { kick: string; valid: boolean }[] {
  const results: { kick: string; valid: boolean }[] = [];
  const lines = stdout.split('\n');

  for (const line of lines) {
    // Match lines like "KICK_X_Y: OK" or "KICK_X_Y: FAIL"
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

/**
 * Parse CSV text into array of objects.
 */
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
}

export function parseSpin(html: string): SpinEntry[] {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  const results: SpinEntry[] = [];
  const links = doc.querySelectorAll('a[href^="v115@"]');
  let idx = 0;
  for (const link of links) {
    const fumen = link.getAttribute('href') ?? '';
    const parent = link.closest('li') ?? link.parentElement;
    const line = (parent?.textContent ?? link.textContent ?? '').trim();
    const statsMatch = line.match(/clear=(\d+),\s*hole=(\d+),\s*piece=(\d+)/);
    if (!statsMatch) continue;
    const markMatch = line.match(/^\[([OX\-])\]/);
    const mark = (markMatch ? markMatch[1] : '-') as 'O' | 'X' | '-';
    const ops = line.replace(/^\[[OX\-]\]\s*/, '').replace(/\[clear=.*?\]\s*/, '').replace(/<[^>]*>/g, '').trim();
    results.push({ index: idx++, operations: ops, mark, fumen,
      clear: parseInt(statsMatch[1]), hole: parseInt(statsMatch[2]), piece: parseInt(statsMatch[3]) });
  }
  if (results.length > 0) return results;
  for (const line of html.split('\n')) {
    const statsMatch = line.match(/clear=(\d+),\s*hole=(\d+),\s*piece=(\d+)/);
    if (!statsMatch) continue;
    const markMatch = line.match(/^\[([OX\-])\]/);
    const mark = (markMatch ? markMatch[1] : '-') as 'O' | 'X' | '-';
    const ops = line.replace(/^\[[OX\-]\]\s*/, '').replace(/\[clear=.*?\]\s*/, '').trim();
    results.push({ index: idx++, operations: ops, mark, fumen: '',
      clear: parseInt(statsMatch[1]), hole: parseInt(statsMatch[2]), piece: parseInt(statsMatch[3]) });
  }
  return results;
}

/** Count spin solutions per category from HTML sections */
export function getSpinCategoryCounts(html: string): Record<string, number> {
  const doc = new DOMParser().parseFromString(html, 'text/html');
  const counts: Record<string, number> = {};
  const sections = doc.querySelectorAll('section');
  for (const section of sections) {
    const cat = section.id || '';
    if (!cat) continue;
    const divs = section.querySelectorAll('div');
    let count = 0;
    for (const div of divs) {
      if (div.textContent?.match(/\[OX\-\]/)) count++;
    }
    if (count > 0) counts[cat] = count;
  }
  return counts;
}
