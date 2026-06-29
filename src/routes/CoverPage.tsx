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
import type { HoldOption, DropType, SfinderOutput } from '@/types/sfinder';
import { invoke } from '@tauri-apps/api/core';
import { GitMerge, Split } from 'lucide-react';

const EMPTY_FIELD_STR = '_'.repeat(10 * 23);
const EMPTY_GARBAGE_STR = '_'.repeat(10);
const PIECE_TYPES = ['I', 'L', 'O', 'Z', 'T', 'J', 'S'];

interface PieceOperation {
  type: string;
  rotation: string;
  x: number;
  y: number;
}

type CoverLogic = 'or' | 'and';

/** Merge all pages into a single field (for garbage cell support) */
function mergeGarbageField(pages: any[]): string {
  const field = Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR);
  for (const p of pages) {
    if (!p.operation) {
      for (let y = 22; y >= 0; y--) {
        for (let x = 0; x < 10; x++) {
          const cell = p.field.at(x, y);
          if (cell === 'X') field.set(x, y, 'X');
        }
      }
    }
  }
  return field.to_fumen_string(); // or similar
}

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

  const t = useT();
  const ready = javaInfo.installed && jarInfo.found;

  /// Build one tetfu per piece placement.
  /// Each tetfu shows the field BEFORE the piece, with the piece as an operation.
  async function buildTetfus(): Promise<string[]> {
      const tetfus: string[] = [];

      // Collect all page data (from operations or auto-split)
      const allOps: { op: PieceOperation; baseField: any }[] = [];

      for (const p of pages) {
          let ops: PieceOperation[];

          if (p.operation) {
              ops = [{
                  type: p.operation.type,
                  rotation: p.operation.rotation,
                  x: p.operation.x,
                  y: p.operation.y,
              }];
          } else {
              const field = p.field;
              let fieldStr = '';
              for (let y = 22; y >= 0; y--) {
                  for (let x = 0; x < 10; x++) {
                      fieldStr += field.at(x, y);
                  }
              }
              const hasAny = PIECE_TYPES.some((t) => fieldStr.includes(t));
              if (!hasAny) continue;

              const result = await invoke<PieceOperation[]>('auto_split_field', { fieldStr });
              if (!result || result.length === 0) return [];
              ops = result;
          }

          if (ops.length === 0) continue;

          // Build a clean base field with garbage (X) cells preserved
          let baseField = Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR);
          const refPage = pages.find((pp: any) => !pp.operation) || pages[0];
          if (refPage) {
              for (let y = 22; y >= 0; y--) {
                  for (let x = 0; x < 10; x++) {
                      if (refPage.field.at(x, y) === 'X') {
                          baseField.set(x, y, 'X');
                      }
                  }
              }
          }

          // For each operation, generate a tetfu with field=BEFORE placement
          for (const op of ops) {
              const encodePages: EncodePage[] = [{
                  field: baseField.copy(),
                  comment: `${op.type}-${op.rotation}`,
                  operation: { type: op.type as any, rotation: op.rotation as any, x: op.x, y: op.y },
              }];
              tetfus.push(encoder.encode(encodePages));

              // Apply piece for the next iteration's base field
              try { baseField.fill({ type: op.type, rotation: op.rotation, x: op.x, y: op.y } as any); } catch {}
          }
      }

      return tetfus;
  }

  const runCover = useCallback(async () => {
    try {
      const tetfus = await buildTetfus();
      if (tetfus.length === 0) {
        useCommandStore.getState().setError(t('cover.splitImpossible'));
        return;
      }

      execute({
        command: 'cover',
        tetfu: tetfus,
        patterns, hold, drop,
        kicks: kicksPath,
        mode: mode || undefined,
        page: 1, clearLine,
      });
    } catch (err: any) {
      useCommandStore.getState().setError(
        typeof err === 'string' ? err : t('cover.splitImpossible'),
      );
    }
  }, [pages, editorFumen, patterns, hold, drop, kicksPath, mode, clearLine, execute, t]);

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
              <option value="1L">1L</option>
              <option value="2L">2L</option>
              <option value="3L">3L</option>
              <option value="4L">4L</option>
              <option value="tetris">Tetris</option>
              <option value="tetris-end">Tetris-end</option>
              <option value="tsm">TSS/TSD/TST/Mini</option>
              <option value="tss">TSS/TSD/TST</option>
              <option value="tsd">TSD/TST</option>
              <option value="tst">TST</option>
              <option value="b2b">B2B</option>
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

      {/* Execute — same runner for both logics */}
      <CommandRunner
        status={status}
        onExecute={runCover}
        onCancel={() => {}}
        disabled={!ready || !editorFumen || !patterns}
      />

      {status.type === 'success' && (
        <OutputViewer output={status.output} command="cover" coverLogic={coverLogic} />
      )}
    </div>
  );
}
