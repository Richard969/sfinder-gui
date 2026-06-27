import { useState, useEffect, useMemo } from 'react';
import type { SfinderOutput } from '@/types/sfinder';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { decoder, encoder } from 'tetris-fumen';
import RawOutput from './RawOutput';
import PercentDisplay from './PercentDisplay';
import { Search } from 'lucide-react';
import { useT } from '@/i18n/useTranslation';

interface OutputViewerProps {
  output: SfinderOutput;
  command: string;
}

interface Solution {
  operations: string;
  fumen?: string;
  cover?: string;
  section: 'unique' | 'minimal';
}

function parseSolutions(html: string): { unique: Solution[]; minimal: Solution[]; allFumen?: string; minimalFumen?: string } {
  let allSolutionsFumen: string | undefined;
  let minimalSolutionsFumen: string | undefined;
  let allSolutionsCount = 0;
  const allSolutions: { pos: number; fumen: string; text: string; cover?: string }[] = [];

  // Split by <h2> sections (sfinder uses h2 for unique/minimal sections)
  const sectionPositions: number[] = [];
  const h2Regex = /<h2[^>]*>/gi;
  let m;
  while ((m = h2Regex.exec(html)) !== null) {
    sectionPositions.push(m.index);
  }
  // If no h2 sections, fallback to "All solutions" markers
  if (sectionPositions.length < 2) {
    const markerRegex = /all\s+solutions/gi;
    while ((m = markerRegex.exec(html)) !== null) {
      sectionPositions.push(m.index);
    }
  }

  // Extract all fumen links: <a href='...v115@CODE'>TEXT</a> (single or double quotes)
  const linkRegex = /<a\s[^>]*href=['"][^'"]*(v115@[^'"\s]+)['"][^>]*>([^<]+)<\/a>/gi;
  let lm;
  while ((lm = linkRegex.exec(html)) !== null) {
    const fumen = lm[1];
    let linkText = lm[2].trim();
    // Capture "All solutions" fumen separately
    if (/all\s+solutions/i.test(linkText)) {
      allSolutionsCount++;
      if (allSolutionsCount === 1) allSolutionsFumen = fumen;
      else if (allSolutionsCount === 2) minimalSolutionsFumen = fumen;
      continue;
    }
    // Skip if it's just Japanese heading text (no piece names)
    if (!/[TIOSZJL]\s*[-]/.test(linkText)) continue;

    // Clean up operation text — remove leading Japanese/HTML noise
    // Format is typically: "T-Reverse O-Spawn S-Spawn J-Reverse L-Reverse I-Spawn"
    const opMatch = linkText.match(/((?:[TIOSZJL]-[A-Za-z]+\s*)+)/);
    if (opMatch) linkText = opMatch[1].trim();

    // Get text after the link for cover %
    const after = html.substring(lm.index + lm[0].length, lm.index + lm[0].length + 100).replace(/<[^>]+>/g, ' ');
    const coverMatch = after.match(/([\d.]+)\s*%/);
    allSolutions.push({
      pos: lm.index,
      fumen,
      text: linkText,
      cover: coverMatch ? `${coverMatch[1]}%` : undefined,
    });
  }

  // Also try bare v115@ codes not in <a> tags (fallback)
  if (allSolutions.length === 0) {
    const fumenRegex = /(v115@[^\s"'<>]+)/g;
    let fm;
    while ((fm = fumenRegex.exec(html)) !== null) {
      const context = html.substring(Math.max(0, fm.index - 50), Math.min(html.length, fm.index + fm[1].length + 150))
        .replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
      const opNames = context.split('/')[0]?.trim() || fm[1].substring(0, 20);
      const coverMatch = context.match(/([\d.]+)\s*%/);
      allSolutions.push({ pos: fm.index, fumen: fm[1], text: opNames, cover: coverMatch ? `${coverMatch[1]}%` : undefined });
    }
  }

  // Split by markers
  const unique: Solution[] = [];
  const minimal: Solution[] = [];

  for (const sol of allSolutions) {
    // After 1st h2 = unique, after 2nd h2 = minimal
    let section: 'unique' | 'minimal' = 'unique';
    if (sectionPositions.length >= 2 && sol.pos > sectionPositions[1]) {
      section = 'minimal';
    } else if (sectionPositions.length >= 1 && sol.pos > sectionPositions[0]) {
      section = 'unique';
    }

    const entry: Solution = {
      operations: sol.text,
      fumen: sol.fumen,
      cover: sol.cover,
      section,
    };

    if (section === 'minimal') minimal.push(entry);
    else unique.push(entry);
  }

  return { unique, minimal, allFumen: allSolutionsFumen, minimalFumen: minimalSolutionsFumen };
}

interface CsvPathRow {
  pattern: string;
  coverage: number;
  used: string;
  unused: string;
  fumens: string[];
}

function parsePathCsv(csv: string): CsvPathRow[] {
  const lines = csv.trim().split('\n');
  if (lines.length < 2) return [];
  const rows: CsvPathRow[] = [];
  for (const line of lines.slice(1)) {
    const cols = line.split(',');
    if (cols.length < 5) continue;
    const coverage = parseInt(cols[1]) || 0;
    const fumenStr = cols[4]?.trim() || '';
    const fumens = fumenStr ? fumenStr.split(';').filter(Boolean) : [];
    rows.push({
      pattern: cols[0].trim(),
      coverage,
      used: cols[2]?.trim() || '',
      unused: cols[3]?.trim() || '',
      fumens,
    });
  }
  return rows;
}

function PathSummary({ total, minimal, allFumen, minFumen, onView, t }: { total: number; minimal: number; allFumen?: string; minFumen?: string; onView: (f: string) => void; t: (k: string) => string }) {
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded border border-border bg-background p-4 text-center">
          <div className="text-3xl font-bold text-primary">{total}</div>
          <div className="text-[11px] text-muted-foreground mt-1">{t('output.allSolutions')}</div>
        </div>
        <div className="rounded border border-border bg-background p-4 text-center">
          <div className="text-3xl font-bold text-primary">{minimal}</div>
          <div className="text-[11px] text-muted-foreground mt-1">{t('output.minimal')}</div>
        </div>
      </div>
      <div className="space-y-2">
        {allFumen && (
          <button onClick={() => onView(allFumen!)}
            className="w-full rounded-md bg-primary/15 px-4 py-2.5 text-sm font-medium text-primary hover:bg-primary/25 transition-colors">
            {t('output.viewAllSolutions')}
          </button>
        )}
        {minimal > 0 && (
          <button onClick={() => onView(minFumen || allFumen!)}
            className="w-full rounded-md bg-primary/15 px-4 py-2.5 text-sm font-medium text-primary hover:bg-primary/25 transition-colors">
            {t('output.viewMinimalSolutions')}
          </button>
        )}
      </div>
    </div>
  );
}

type TabId = 'summary' | 'solutions' | 'strict-minimal' | 'stdout' | 'csv' | 'stderr';

function parseStdoutCoverage(stdout: string): { pct: string; fraction: string } | null {
  // Match: success = 100.00% (10080/10080) or similar
  const m = stdout.match(/(\d+\.?\d*)\s*%\s*\((\d+)\s*\/\s*(\d+)\)/);
  if (m) return { pct: `${m[1]}%`, fraction: `${m[2]}/${m[3]}` };
  // Match: 84.64% (711/840)
  const m2 = stdout.match(/(\d+\.?\d*)\s*%\s*\((\d+)\/(\d+)\)/);
  if (m2) return { pct: `${m2[1]}%`, fraction: `${m2[2]}/${m2[3]}` };
  return null;
}

/** Combine multiple fumen solutions into a single multi-page fumen with coverage comments */
function combineFumens(items: { fumen: string; coverage: number }[], totalPatterns: number): string | null {
  try {
    const allPages: any[] = [];
    for (const item of items) {
      const pct = (item.coverage / totalPatterns * 100).toFixed(2);
      const comment = `Covered patterns(${item.coverage}/${totalPatterns}) (${pct}%)`;
      const pages = decoder.decode(item.fumen.startsWith('v115@') ? item.fumen : `v115@${item.fumen}`);
      // Take only the LAST page (final field state) — ignores split steps
      const lastPage = pages[pages.length - 1];
      if (lastPage) {
        allPages.push({
          field: lastPage.field,
          comment,
        });
      }
    }
    if (allPages.length === 0) return null;
    return encoder.encode(allPages);
  } catch {
    return null;
  }
}

function PathCsvSummary({ rows, t, stdout, minimalRows, onView, totalPatterns }: { rows: { fumen: string; coverage: number; used: string }[]; t: (k: string) => string; stdout: string; minimalRows: { fumen: string; coverage: number; used: string }[]; onView: (f: string) => void; totalPatterns: number }) {
  const cov = parseStdoutCoverage(stdout);
  const allCombined = useMemo(() => combineFumens(rows.map((r) => ({ fumen: r.fumen, coverage: r.coverage })), totalPatterns), [rows, totalPatterns]);
  const minimalCombined = useMemo(() => combineFumens(minimalRows.map((r) => ({ fumen: r.fumen, coverage: r.coverage })), totalPatterns), [minimalRows, totalPatterns]);

  return (
    <div className="space-y-4">
      {cov && (
        <div className="rounded border border-primary/30 bg-primary/5 p-4 text-center">
          <div className="text-4xl font-bold text-primary">{cov.pct}</div>
          <div className="text-xs text-muted-foreground mt-1">{cov.fraction} sequences successful</div>
        </div>
      )}
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded border border-border bg-background p-4 text-center">
          <div className="text-3xl font-bold text-primary">{rows.length}</div>
          <div className="text-[11px] text-muted-foreground mt-1">Unique Solutions</div>
        </div>
        <div className="rounded border border-border bg-background p-4 text-center">
          <div className="text-3xl font-bold text-primary">{minimalRows.length}</div>
          <div className="text-[11px] text-muted-foreground mt-1">Minimal Solutions</div>
        </div>
      </div>
      <div className="space-y-2">
        {allCombined && (
          <button onClick={() => onView(allCombined!)}
            className="w-full rounded-md bg-primary/15 px-4 py-2.5 text-sm font-medium text-primary hover:bg-primary/25 transition-colors">
            {t('output.viewAllSolutions')}
          </button>
        )}
        {minimalCombined && (
          <button onClick={() => onView(minimalCombined!)}
            className="w-full rounded-md bg-primary/15 px-4 py-2.5 text-sm font-medium text-primary hover:bg-primary/25 transition-colors">
            {t('output.viewMinimalSolutions')}
          </button>
        )}
      </div>
    </div>
  );
}

function PathCsvTable({ rows, onView, t, totalPatterns }: { rows: { fumen: string; coverage: number; used: string }[]; t: (k: string) => string; onView: (f: string) => void; totalPatterns: number }) {
  const [filter, setFilter] = useState('');
  const filtered = useMemo(
    () => filter ? rows.filter((r) => r.used.toUpperCase().includes(filter.toUpperCase())) : rows,
    [rows, filter],
  );
  return (
    <div className="space-y-2">
      <div className="relative">
        <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
        <input type="text" value={filter} onChange={(e) => setFilter(e.target.value)}
          placeholder={t('output.filter')}
          className="w-full rounded border border-input bg-background pl-7 pr-2 py-1 text-xs
            placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-ring" />
      </div>
      <div className="rounded-md border border-border overflow-hidden max-h-[400px] overflow-y-auto">
        <table className="w-full text-xs">
          <thead className="bg-secondary/50 sticky top-0">
            <tr>
              <th className="px-2 py-1.5 text-right font-medium text-muted-foreground w-20">Coverage</th>
              <th className="px-2 py-1.5 text-left font-medium text-muted-foreground">Used</th>
              <th className="px-2 py-1.5 text-center font-medium text-muted-foreground w-16">View</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {filtered.map((row, i) => (
              <tr key={i} className="hover:bg-secondary/30">
                <td className="px-2 py-1 text-right">
                  <span className="text-green-400 font-bold">{(row.coverage / totalPatterns * 100).toFixed(1)}%</span>
                </td>
                <td className="px-2 py-1 font-mono text-muted-foreground">{row.used || '-'}</td>
                <td className="px-2 py-1 text-center">
                  <button onClick={() => onView(row.fumen)}
                    className="text-[10px] px-2 py-0.5 rounded bg-primary/15 text-primary hover:bg-primary/25 font-medium">
                    {t('output.view')}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export default function OutputViewer({ output, command }: OutputViewerProps) {
  const t = useT();
  const [activeTab, setActiveTab] = useState<TabId>(output.exitCode !== 0 ? 'stderr' : 'summary');
  const [fileContents, setFileContents] = useState<Record<string, string>>({});
  const [search, setSearch] = useState('');

  useEffect(() => {
    async function read() {
      const contents: Record<string, string> = {};
      for (const file of output.outputFiles) {
        try {
          contents[file] = await invoke<string>('read_output_file', { path: file });
        } catch { contents[file] = ''; }
      }
      setFileContents(contents);
    }
    read();
  }, [output.outputFiles]);

  const htmlOutput = Object.values(fileContents).find((c) => c.length > 50) || '';
  const csvFileContent = Object.values(fileContents).find((c) => c.includes('テト譜')) || '';
  const { unique, minimal, allFumen, minimalFumen } = useMemo(() => parseSolutions(htmlOutput), [htmlOutput]);
  const pathRows = useMemo(() => {
    return (output.pathResults || []).map((r) => ({ fumen: r.fumen, coverage: r.coverage, used: r.used }));
  }, [output.pathResults]);
  const strictMinimalRows = useMemo(() => {
    return (output.strictMinimal || []).map((r) => ({ fumen: r.fumen, coverage: r.coverage, used: r.used }));
  }, [output.strictMinimal]);
  const pathTotalPatterns = output.pathTotalPatterns || pathRows.length || 1;

  const handleView = (fumen: string) => {
    try {
      const base = window.location.origin;
      const url = `${base}/view-fumen?fumen=${encodeURIComponent(fumen)}`;
      const win = new WebviewWindow(`fumen-${Date.now()}`, {
        url,
        title: 'Fumen Viewer',
        width: 720,
        height: 900,
        resizable: true,
        center: true,
      });
      win.once('tauri://error', (e) => {
        console.error('View window error:', e);
      });
    } catch (e) {
      console.error('Failed to create view window:', e);
    }
  };

  const SolutionTable = ({ solutions, label }: { solutions: Solution[]; label: string }) => {
    const filtered = useMemo(
      () => search ? solutions.filter((s) => s.operations.toLowerCase().includes(search.toLowerCase())) : solutions,
      [solutions, search],
    );

    if (solutions.length === 0) {
      return <p className="text-sm text-muted-foreground">{t('output.noSolutions')}</p>;
    }

    return (
      <div className="space-y-2">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
          <input type="text" value={search} onChange={(e) => setSearch(e.target.value)}
            placeholder={t('output.filter')}
            className="w-full rounded border border-input bg-background pl-7 pr-2 py-1 text-xs
              placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-ring" />
        </div>
        <div className="rounded-md border border-border overflow-hidden max-h-[400px] overflow-y-auto">
          <table className="w-full text-xs">
            <thead className="bg-secondary/50 sticky top-0">
              <tr>
                <th className="px-2 py-1.5 text-left font-medium text-muted-foreground w-8">#</th>
                <th className="px-2 py-1.5 text-left font-medium text-muted-foreground">{t('output.operations')}</th>
                <th className="px-2 py-1.5 text-right font-medium text-muted-foreground w-20">{t('output.cover')}</th>
                <th className="px-2 py-1.5 text-center font-medium text-muted-foreground w-16">{t('output.view')}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {filtered.map((s, i) => (
                <tr key={i} className="hover:bg-secondary/30">
                  <td className="px-2 py-1 text-muted-foreground">{i + 1}</td>
                  <td className="px-2 py-1 font-mono">{s.operations}</td>
                  <td className="px-2 py-1 text-right text-muted-foreground">{s.cover ?? '-'}</td>
                  <td className="px-2 py-1 text-center">
                    {s.fumen && (
                      <button onClick={() => handleView(s.fumen!)}
                        className="text-[10px] px-2 py-0.5 rounded bg-primary/15 text-primary hover:bg-primary/25 font-medium">
                        {t('output.view')}
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  };

  const tabs: { id: TabId; label: string }[] = [];
  const failed = output.exitCode !== 0;

  if (failed) {
    tabs.push({ id: 'stderr', label: t('output.stderr') });
  } else if (command === 'path' && pathRows.length > 0) {
    tabs.push({ id: 'summary', label: t('output.summary') });
    tabs.push({ id: 'solutions', label: `${t('output.solutions')} (${pathRows.length})` });
    if (strictMinimalRows.length > 0) tabs.push({ id: 'strict-minimal', label: `Strict Minimal (${strictMinimalRows.length})` });
    tabs.push({ id: 'csv', label: 'CSV' });
    if (output.stderr) tabs.push({ id: 'stderr', label: t('output.stderr') });
  } else if (command === 'percent') {
    tabs.push({ id: 'summary', label: t('output.summary') });
    tabs.push({ id: 'stdout', label: t('output.stdout') });
    if (output.stderr) tabs.push({ id: 'stderr', label: t('output.stderr') });
  } else {
    tabs.push({ id: 'summary', label: t('output.summary') });
    if (unique.length + minimal.length > 0) tabs.push({ id: 'solutions', label: `${t('output.allSolutions')} (${unique.length + minimal.length})` });
    tabs.push({ id: 'stdout', label: t('output.stdout') });
    if (htmlOutput) tabs.push({ id: 'csv', label: t('output.rawHtml') });
    if (output.stderr) tabs.push({ id: 'stderr', label: t('output.stderr') });
  }

  return (
    <div className="rounded-lg border border-border bg-card overflow-hidden">
      <div className="flex border-b border-border bg-secondary/30 overflow-x-auto">
        {tabs.map((tab) => (
          <button key={tab.id} onClick={() => setActiveTab(tab.id)}
            className={`px-3 py-2 text-xs font-medium transition-colors shrink-0
              ${activeTab === tab.id
                ? 'border-b-2 border-primary text-foreground bg-card'
                : 'text-muted-foreground hover:text-foreground'
              }`}>
            {tab.label}
          </button>
        ))}
      </div>

      <div className="p-4 max-h-[600px] overflow-y-auto">
        {failed && activeTab === 'stderr' && (
          <div className="space-y-3">
            <div className="text-sm text-red-400 font-medium">{t('output.exit')}: {output.exitCode}</div>
            <RawOutput text={output.stderr || output.stdout || t('output.empty')} />
          </div>
        )}
        {!failed && activeTab === 'summary' && command === 'percent' && (
          <PercentDisplay stdout={output.stdout} />
        )}
        {!failed && activeTab === 'summary' && command === 'path' && (
          <PathCsvSummary rows={pathRows} t={t} stdout={output.stdout} minimalRows={strictMinimalRows} onView={handleView} totalPatterns={pathTotalPatterns} />
        )}
        {!failed && activeTab === 'summary' && command !== 'percent' && command !== 'path' && (
          <PathSummary total={unique.length + minimal.length} minimal={minimal.length} allFumen={allFumen} minFumen={minimalFumen} onView={handleView} t={t} />
        )}
        {!failed && activeTab === 'solutions' && command === 'path' && (
          <PathCsvTable rows={pathRows} onView={handleView} t={t} totalPatterns={pathTotalPatterns} />
        )}
        {!failed && activeTab === 'strict-minimal' && (
          <PathCsvTable rows={strictMinimalRows} onView={handleView} t={t} totalPatterns={pathTotalPatterns} />
        )}
        {!failed && activeTab === 'solutions' && command !== 'path' && (
          <SolutionTable solutions={[...unique, ...minimal]} label="all" />
        )}
        {!failed && activeTab === 'stdout' && <RawOutput text={output.stdout || '(empty)'} />}
        {!failed && activeTab === 'csv' && <RawOutput text={htmlOutput || output.stdout || '(empty)'} />}
        {!failed && activeTab === 'stderr' && <RawOutput text={output.stderr || '(empty)'} />}
      </div>

      <div className="flex items-center border-t border-border px-4 py-2 text-xs text-muted-foreground">
        <span className="font-mono truncate">{output.commandLine}</span>
        {output.exitCode !== 0 && (
          <span className="text-red-400 shrink-0 ml-auto">{t('output.exit')}: {output.exitCode}</span>
        )}
      </div>
    </div>
  );
}
