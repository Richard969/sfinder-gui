import { useState, useEffect, useCallback } from 'react';
import { useAppStore } from '@/stores/appStore';
import { useCommandStore } from '@/stores/commandStore';
import { useSfinderCommand } from '@/hooks/useSfinderCommand';
import { useEditorFumen } from '@/components/fumen/FumenEditorEmbed';
import { useFumenStore } from '@/stores/fumenStore';
import { useT } from '@/i18n/useTranslation';
import { Field, encoder } from 'tetris-fumen';
import type { EncodePage } from 'tetris-fumen';
import FumenEditorEmbed from '@/components/fumen/FumenEditorEmbed';
import PatternInput from '@/components/forms/PatternInput';
import CommandOptions from '@/components/forms/CommandOptions';
import CommandRunner from '@/components/forms/CommandRunner';
import OutputViewer from '@/components/output/OutputViewer';
import type { HoldOption, DropType, SfinderOutput, CoverResultEntry } from '@/types/sfinder';
import { parseCoverage } from '@/lib/output-parser';
import { invoke } from '@tauri-apps/api/core';
import { GitMerge, Split } from 'lucide-react';

const EMPTY_FIELD_STR = '_'.repeat(10 * 23);
const EMPTY_GARBAGE_STR = '_'.repeat(10);

interface PieceOperation {
  type: string;
  rotation: string;
  x: number;
  y: number;
}

type CoverLogic = 'or' | 'and';

export default function CoverPage() {
  const jarInfo = useAppStore((s) => s.sfinderJarInfo);
  const javaInfo = useAppStore((s) => s.javaInfo);
  const status = useCommandStore((s) => s.status);
  const clearStatus = useCommandStore((s) => s.clearStatus);
  const execute = useSfinderCommand();
  useEffect(() => { clearStatus(); }, [clearStatus]);

  const editorFumen = useEditorFumen();
  const patterns = useFumenStore((s) => s.patterns);
  const setPatterns = useFumenStore((s) => s.setPatterns);
  const pages = useFumenStore((s) => s.pages);
  const currentPageIndex = useFumenStore((s) => s.currentPageIndex);
  const page = currentPageIndex + 1;

  const [hold, setHold] = useState<HoldOption>('use');
  const [drop, setDrop] = useState<DropType>('softdrop');
  const [kicksPath, setKicksPath] = useState('srs');
  const clearLine = useFumenStore((s) => s.clearLine);
  const setClearLine = useFumenStore((s) => s.setClearLine);
  const [mode, setMode] = useState('normal');
  const [coverLogic, setCoverLogic] = useState<CoverLogic>('or');

  // AND mode state
  const [andRunning, setAndRunning] = useState(false);
  const [andProgress, setAndProgress] = useState({ current: 0, total: 0 });
  const [andOutput, setAndOutput] = useState<SfinderOutput | null>(null);

  const t = useT();
  const ready = javaInfo.installed && jarInfo.found;
  const pageCount = pages.length;

  const PIECE_TYPES = ['I', 'L', 'O', 'Z', 'T', 'J', 'S'];

  /// For each page: if no operation and has cells → auto-split via Rust.
  /// Collect all operations into one flat array, generate combined fumen.
  /// Does NOT modify fumenStore pages.
  async function buildTetfu(): Promise<string | null> {
    const allOps: PieceOperation[] = [];

    for (const p of pages) {
      if (p.operation) {
        // Already piece-by-piece — use operation directly
        allOps.push({
          type: p.operation.type,
          rotation: p.operation.rotation,
          x: p.operation.x,
          y: p.operation.y,
        });
      } else {
        // Final field state — auto-split
        const field = p.field;
        let fieldStr = '';
        for (let y = 22; y >= 0; y--) {
          for (let x = 0; x < 10; x++) {
            fieldStr += field.at(x, y);
          }
        }

        // Skip empty pages
        const hasAny = PIECE_TYPES.some((t) => fieldStr.includes(t));
        if (!hasAny) continue;

        const ops = await invoke<PieceOperation[]>('auto_split_field', { fieldStr });
        if (!ops || ops.length === 0) return null;
        allOps.push(...ops);
      }
    }

    if (allOps.length === 0) return null;

    // Pre-populate garbage (X) cells from the original field so they act as support
    let currentField = Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR);
    const firstPage = pages.find((p) => !p.operation) || pages[0];
    if (firstPage) {
      for (let y = 22; y >= 0; y--) {
        for (let x = 0; x < 10; x++) {
          if (firstPage.field.at(x, y) === 'X') {
            currentField.set(x, y, 'X');
          }
        }
      }
    }

    const encodePages: EncodePage[] = [];
    for (const op of allOps) {
      try {
        currentField.fill({ type: op.type, rotation: op.rotation, x: op.x, y: op.y } as any);
      } catch { continue; }
      encodePages.push({
        field: currentField.copy(),
        comment: `${op.type}-${op.rotation}`,
        operation: { type: op.type as any, rotation: op.rotation as any, x: op.x, y: op.y },
      });
    }

    if (encodePages.length === 0) return null;
    return encoder.encode(encodePages);
  }

  // OR mode: build tetfu (auto-split if needed), execute cover
  const runOrMode = useCallback(async () => {
    setAndOutput(null);
    try {
      const tetfu = await buildTetfu();
      if (!tetfu) {
        useCommandStore.getState().setError(t('cover.splitImpossible'));
        return;
      }
      execute({
        command: 'cover', tetfu, patterns, hold, drop,
        kicks: kicksPath, mode: mode || undefined, coverLogic: 'or', page: 1, clearLine,
      });
    } catch (err: any) {
      useCommandStore.getState().setError(
        typeof err === 'string' ? err : t('cover.splitImpossible'),
      );
    }
  }, [pages, editorFumen, patterns, hold, drop, kicksPath, mode, clearLine, execute, t]);

  // AND mode: build tetfu, run cover per page, compute intersection
  const runAndMode = useCallback(async () => {
    setAndOutput(null);

    let tetfu: string;
    try {
      const result = await buildTetfu();
      if (!result) {
        useCommandStore.getState().setError(t('cover.splitImpossible'));
        return;
      }
      tetfu = result;
    } catch (err: any) {
      useCommandStore.getState().setError(
        typeof err === 'string' ? err : t('cover.splitImpossible'),
      );
      return;
    }

    // Decode the generated fumen to get individual pages
    let workPages: { field: any; comment: string; operation?: any }[];
    try {
      const { decoder } = await import('tetris-fumen');
      const decoded = decoder.decode(tetfu);
      workPages = decoded.map((dp: any) => ({
        field: dp.field,
        comment: dp.comment ?? '',
        operation: dp.operation
          ? { type: dp.operation.type, rotation: dp.operation.rotation, x: dp.operation.x, y: dp.operation.y }
          : undefined,
      }));
    } catch {
      // Fallback: use original pages
      workPages = pages;
    }

    if (workPages.length <= 1) {
      execute({
        command: 'cover', tetfu, patterns, hold, drop,
        kicks: kicksPath, mode: mode || undefined, coverLogic: 'or', page: 1, clearLine,
      });
      return;
    }

    setAndRunning(true);
    setAndProgress({ current: 0, total: workPages.length });

    const settings = useAppStore.getState().settings;
    const perPageResults: { pageIndex: number; stdout: string; coverResults: CoverResultEntry[]; totalPatterns: number }[] = [];

    for (let i = 0; i < workPages.length; i++) {
      setAndProgress({ current: i + 1, total: workPages.length });

      const p = workPages[i];
      const singleFumen = encoder.encode([{
        field: p.field,
        comment: p.comment || undefined,
        operation: p.operation
          ? { type: p.operation.type as any, rotation: p.operation.rotation as any, x: p.operation.x, y: p.operation.y }
          : undefined,
      }]);

      try {
        const output = await invoke<SfinderOutput>('run_sfinder_command', {
          config: {
            command: 'cover',
            tetfu: singleFumen,
            page: 1,
            clearLine,
            patterns,
            hold,
            drop,
            kicks: kicksPath,
            mode: mode || undefined,
            jarPath: settings.sfinderJarPath,
            javaPath: settings.javaPath,
          },
        });

        perPageResults.push({
          pageIndex: i,
          stdout: output.stdout,
          coverResults: output.coverResults || [],
          totalPatterns: output.coverTotalPatterns || 0,
        });
      } catch (err: any) {
        perPageResults.push({
          pageIndex: i,
          stdout: `Error: ${String(err)}`,
          coverResults: [],
          totalPatterns: 0,
        });
      }
    }

    // Compute AND intersection
    const allPatternSets = perPageResults
      .filter((r) => r.coverResults.length > 0)
      .map((r) => new Set(r.coverResults.map((c) => c.pattern)));

    let intersectionPatterns: Set<string> | null = null;
    if (allPatternSets.length > 0) {
      intersectionPatterns = allPatternSets[0];
      for (let i = 1; i < allPatternSets.length; i++) {
        intersectionPatterns = new Set(
          [...intersectionPatterns].filter((p) => allPatternSets[i].has(p))
        );
      }
    }

    const totalPatterns = perPageResults.find((r) => r.totalPatterns > 0)?.totalPatterns || 0;
    const intersectionCount = intersectionPatterns?.size || 0;
    const andPct = totalPatterns > 0 ? ((intersectionCount / totalPatterns) * 100) : 0;

    const stdoutLines: string[] = [];
    for (const pr of perPageResults) {
      const cov = parseCoverage(pr.stdout);
      if (cov) {
        stdoutLines.push(`Page ${pr.pageIndex + 1}: OR = ${cov.overallRatio.toFixed(2)} % [${cov.numerator}/${cov.denominator}]`);
      } else {
        stdoutLines.push(`Page ${pr.pageIndex + 1}: (failed)`);
      }
    }
    stdoutLines.push('');
    stdoutLines.push(`AND = ${andPct.toFixed(2)} % [${intersectionCount}/${totalPatterns}]`);

    const intersectionResults: CoverResultEntry[] = [];
    if (intersectionPatterns && totalPatterns > 0) {
      const firstResults = perPageResults.find((r) => r.coverResults.length > 0);
      if (firstResults) {
        for (const pattern of intersectionPatterns) {
          const ref = firstResults.coverResults.find((c) => c.pattern === pattern);
          intersectionResults.push({
            pattern,
            fumen: ref?.fumen || '',
            coverage: ref?.coverage || 0,
            used: ref?.used || '',
          });
        }
      }
    }

    const syntheticOutput: SfinderOutput = {
      stdout: stdoutLines.join('\n'),
      stderr: '',
      exitCode: 0,
      outputFiles: [],
      commandLine: `AND mode — ${workPages.length} pages`,
      coverResults: intersectionResults,
      coverTotalPatterns: totalPatterns,
    };

    setAndOutput(syntheticOutput);
    setAndRunning(false);
  }, [pages, patterns, hold, drop, kicksPath, mode, clearLine, execute, t]);

  return (
    <div className="max-w-5xl mx-auto space-y-4">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold">{t('cover.title')}</h2>
        <p className="text-sm text-muted-foreground">{t('cover.desc')}</p>
      </div>

      <FumenEditorEmbed visibleRows={clearLine} onVisibleRowsChange={setClearLine} />

      <PatternInput value={patterns} onChange={setPatterns} />

      <CommandOptions
        hold={hold} onHoldChange={setHold}
        drop={drop} onDropChange={setDrop}
        kicksPath={kicksPath} onKicksPathChange={setKicksPath}
      />

      {/* Mode selector + OR/AND toggle */}
      <div className="space-y-3">
        <div className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
          {t('cover.modeLabel')}
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">{t('cover.mode')}</label>
            <select
              value={mode}
              onChange={(e) => setMode(e.target.value)}
              className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-sm
                focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <option value="normal">{t('cover.normal')}</option>
              <option value="tspin">{t('cover.tspin')}</option>
            </select>
          </div>

          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">{t('cover.coverLogic')}</label>
            <div className="flex rounded-md border border-input overflow-hidden">
              <button
                onClick={() => setCoverLogic('or')}
                className={`flex-1 flex items-center justify-center gap-1 px-2.5 py-1.5 text-xs font-medium transition-colors
                  ${coverLogic === 'or' ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}
              >
                <GitMerge className="h-3 w-3" />
                {t('cover.orLogic')}
              </button>
              <button
                onClick={() => setCoverLogic('and')}
                className={`flex-1 flex items-center justify-center gap-1 px-2.5 py-1.5 text-xs font-medium transition-colors
                  ${coverLogic === 'and' ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}
              >
                <Split className="h-3 w-3" />
                {t('cover.andLogic')}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Execute */}
      {coverLogic === 'or' && (
        <CommandRunner
          status={andOutput ? { type: 'success', output: andOutput } : status}
          onExecute={runOrMode}
          onCancel={() => {}}
          disabled={!ready || !editorFumen || !patterns}
        />
      )}

      {coverLogic === 'and' && (
        <CommandRunner
          status={andRunning
            ? { type: 'running', startTime: Date.now() }
            : andOutput
              ? { type: 'success', output: andOutput }
              : status}
          onExecute={runAndMode}
          onCancel={() => {}}
          disabled={!ready || !editorFumen || !patterns || andRunning}
        />
      )}

      {andRunning && (
        <div className="text-xs text-muted-foreground text-center">
          {t('cover.andCoverage')}: {andProgress.current} / {andProgress.total}
        </div>
      )}

      {status.type === 'success' && coverLogic === 'or' && !andOutput && (
        <OutputViewer output={status.output} command="cover" />
      )}
      {andOutput && (
        <OutputViewer output={andOutput} command="cover" coverLogic="and" />
      )}
    </div>
  );
}
