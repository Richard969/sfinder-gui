import { useState, useEffect, useCallback } from 'react';
import { useAppStore } from '@/stores/appStore';
import { useCommandStore } from '@/stores/commandStore';
import { useSfinderCommand } from '@/hooks/useSfinderCommand';
import { useEditorFumen } from '@/components/fumen/FumenEditorEmbed';
import { useFumenStore } from '@/stores/fumenStore';
import { useDisplayStore } from '@/stores/displayStore';
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
  const rows = useDisplayStore((s) => s.rows);
  const setRows = useDisplayStore((s) => s.setRows);
  const [mode, setMode] = useState('normal');
  const [coverLogic, setCoverLogic] = useState<CoverLogic>('or');
  const [trimWarning, setTrimWarning] = useState<string | null>(null);

  const t = useT();
  const ready = javaInfo.installed && jarInfo.found;

  /// Build one tetfu per page.
  /// Each tetfu = one placement pattern (may be multi-page if auto-split).
  async function buildTetfus(): Promise<string[]> {
    const maxRows = rows + 4;
    setTrimWarning(null);
    const tetfus: string[] = [];

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

      // Start from the page's actual field (only garbage cells remain as blocks)
      let currentField = Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR);
      const refPage = pages.find((pp: any) => !pp.operation) || pages[0];
      if (refPage) {
        for (let y = 22; y >= 0; y--) {
          for (let x = 0; x < 10; x++) {
            if (refPage.field.at(x, y) === 'X') {
              currentField.set(x, y, 'X');
            }
          }
        }
      }

      // Find the bottom row that has any block (garbage or existing)
      let bottomRow = 0;
      for (let y = 0; y <= 22; y++) {
        for (let x = 0; x < 10; x++) {
          if (currentField.at(x, y) !== '_') {
            bottomRow = y + 1;
          }
        }
      }

      let height = bottomRow + 1;
      for (const op of ops) {
        const pieceBottom = (op.y || 0) + 2;
        if (pieceBottom > height) height = pieceBottom;
      }

      if (height > maxRows) {
        setTrimWarning(`${t('cover.trimWarning')} (max: ${maxRows}, found: ${height})`);
        height = maxRows;
      }

      const encodePages: EncodePage[] = [];
      for (const op of ops) {
        // DO NOT fill piece blocks — only operation (locked) tells sfinder where it goes
        encodePages.push({
          field: currentField.copy(),
          comment: `${op.type}-${op.rotation}`,
          operation: { type: op.type as any, rotation: op.rotation as any, x: op.x, y: op.y },
        });
      }
      if (encodePages.length === 0) continue;

      // Trim field to height rows
      const trimPages: EncodePage[] = encodePages.map((ep) => {
        const trimmed = Field.create('_'.repeat(10 * height), EMPTY_GARBAGE_STR);
        const src = ep.field;
        if (src) {
          for (let y = 0; y < height; y++) {
            for (let x = 0; x < 10; x++) {
              const cell = src.at(x, y);
              if (cell !== '_') trimmed.set(x, y, cell);
            }
          }
        }
        return { field: trimmed, operation: ep.operation };
      });

      tetfus.push(encoder.encode(trimPages));
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
      <FumenEditorEmbed visibleRows={rows} onVisibleRowsChange={setRows} />

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

      {/* Trim warning */}
      {trimWarning && (
        <div className="bg-yellow-500/10 border border-yellow-500/30 rounded-md px-4 py-2 text-xs text-yellow-600">
          ⚠️ {trimWarning.split('(')[0]}<span className="text-yellow-500"> ({trimWarning.split('(')[1]}</span>
        </div>
      )}

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
