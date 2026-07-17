import { useFumenStore } from '@/stores/fumenStore';
import { Trash2, Undo2, Redo2, ArrowLeftRight, ArrowUpDown, ArrowLeft, Copy, ClipboardPaste, FilePlus, Camera } from 'lucide-react';
import { useCallback, useEffect, useState, useRef } from 'react';
import { useT } from '@/i18n/useTranslation';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Field, encoder } from 'tetris-fumen';

const PIECE_CHARS = new Set(['I', 'O', 'T', 'S', 'Z', 'J', 'L', 'X']);

function fieldStrToFumen(fieldStr: string): string | null {
  let lines = fieldStr.trim().split('\n').filter(Boolean);
  lines = lines.reverse(); // output[0]=visual bottom, rev so top→fumen row 0
  if (lines.length === 0) return null;
  // Pad or truncate each line to exactly 10 chars
  const padded = lines.map(line => {
    if (line.length < 10) return line + '_'.repeat(10 - line.length);
    return line.substring(0, 10);
  });
  try {
    const field = Field.create('_'.repeat(10 * 23), '_'.repeat(10));
    // lines[0] = bottom of board (recognition scans bottom-to-top) = fumen row 0
    for (let row = 0; row < padded.length; row++) {
      const line = padded[row];
      for (let col = 0; col < 10; col++) {
        const ch = line[col];
        if (PIECE_CHARS.has(ch)) field.set(col, row, ch as any);
      }
    }
    return encoder.encode([{ field }]);
  } catch (e) {
    console.error('[fieldStrToFumen] encode error:', e);
    return null;
  }
}

export default function FumenToolbar() {
  const t = useT();
  const [capturing, setCapturing] = useState(false);
  const [toast, setToast] = useState<{ msg: string; type: 'error' | 'success' } | null>(null);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const showToast = useCallback((msg: string, type: 'error' | 'success' = 'error') => {
    setToast({ msg, type });
    if (toastTimer.current) clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(null), 4000);
  }, []);

  const decodeFumen = useFumenStore((s) => s.decodeFumen);
  const newFile = useFumenStore((s) => s.newFile);
  const undo = useFumenStore((s) => s.undo);
  const redo = useFumenStore((s) => s.redo);
  const flipHorizontal = useFumenStore((s) => s.flipHorizontal);
  const flipVertical = useFumenStore((s) => s.flipVertical);
  const mirrorField = useFumenStore((s) => s.mirrorField);
  const fumenString = useFumenStore((s) => s.fumenString);
  const clearField = useFumenStore((s) => s.clearField);
  const undoStack = useFumenStore((s) => s.undoStack);
  const redoStack = useFumenStore((s) => s.redoStack);

  // Listen for screenshot result → load field
  const handleFieldStr = useCallback((fieldStr: string) => {
    console.log('[screenshot] raw fieldStr:', JSON.stringify(fieldStr));
    const fumen = fieldStrToFumen(fieldStr);
    if (fumen) {
      decodeFumen(fumen);
      showToast('Field loaded from screenshot', 'success');
    } else {
      console.log('[screenshot] fieldStrToFumen returned null. Lines:', fieldStr?.trim().split('\n').length, 'first line length:', fieldStr?.trim().split('\n')[0]?.length);
      showToast('Recognition result could not be parsed', 'error');
    }
    setCapturing(false);
  }, [decodeFumen, showToast]);

  useEffect(() => {
    const unlisten = listen<string>('screenshot-result', (event) => {
      handleFieldStr(event.payload);
    });
    return () => { unlisten.then(fn => fn()); };
  }, [handleFieldStr]);

  // Listen for screenshot cancel (Esc / close)
  useEffect(() => {
    const unlisten = listen<string>('screenshot-cancelled', () => {
      setCapturing(false);
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Listen for screenshot error
  useEffect(() => {
    const unlisten = listen<string>('screenshot-error', (event) => {
      showToast(event.payload || 'Screenshot recognition failed', 'error');
      setCapturing(false);
    });
    return () => { unlisten.then(fn => fn()); };
  }, [showToast]);

  const handleCopy = useCallback(async () => {
    try { await navigator.clipboard.writeText(fumenString); } catch { }
  }, [fumenString]);

  const handleImport = useCallback(async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text.trim()) decodeFumen(text);
    } catch { }
  }, [decodeFumen]);

  const handleScreenshot = useCallback(async () => {
    setCapturing(true);
    try {
      await invoke('start_capture');
    } catch (err) {
      console.error('Screenshot capture failed:', err);
      setCapturing(false);
      showToast(String(err), 'error');
    }
  }, [showToast]);

  const actions = [
    { icon: FilePlus, label: t('editor.new'), onClick: newFile },
    { type: 'separator' as const },
    { icon: Camera, label: 'Screenshot', onClick: handleScreenshot },
    { type: 'separator' as const },
    { icon: Undo2, label: t('editor.undo'), onClick: undo, disabled: undoStack.length === 0 },
    { icon: Redo2, label: t('editor.redo'), onClick: redo, disabled: redoStack.length === 0 },
    { type: 'separator' as const },
    { icon: ArrowLeftRight, label: t('editor.flipH'), onClick: flipHorizontal },
    { icon: ArrowUpDown, label: t('editor.flipV'), onClick: flipVertical },
    { icon: ArrowLeft, label: t('editor.mirror'), onClick: mirrorField },
    { type: 'separator' as const },
    { icon: Copy, label: t('editor.copyFumen'), onClick: handleCopy },
    { icon: ClipboardPaste, label: t('editor.importFumen'), onClick: handleImport },
    { type: 'separator' as const },
    { icon: Trash2, label: t('editor.clearField'), onClick: clearField, danger: true },
  ];

  return (
    <>
      <div className="flex items-center gap-1 rounded-lg border border-border bg-card p-1.5 shadow-sm">
        {actions.map((action, i) => {
          if ('type' in action && action.type === 'separator') {
            return <div key={i} className="w-px h-6 bg-border mx-1" />;
          }
          const { icon: Icon, label, onClick, disabled, danger } = action as {
            icon: typeof Trash2;
            label: string;
            onClick: () => void;
            disabled?: boolean;
            danger?: boolean;
          };
          const isScreenshot = label === 'Screenshot';
          return (
            <button
              key={label}
              onClick={onClick}
              disabled={disabled || capturing}
              title={label}
              className={`
                flex items-center justify-center h-8 w-8 rounded-md text-xs transition-colors shrink-0
                ${disabled || capturing
                  ? isScreenshot && capturing
                    ? 'animate-pulse text-primary'
                    : 'text-muted-foreground/30 cursor-not-allowed'
                  : danger
                    ? 'text-muted-foreground hover:bg-red-500/15 hover:text-red-400'
                    : 'text-muted-foreground hover:bg-secondary hover:text-foreground'
                }
              `}
            >
              <Icon className="h-4 w-4" />
            </button>
          );
        })}
      </div>

      {/* Toast */}
      {toast && (
        <div
          className={`
            fixed bottom-4 right-4 z-50 px-4 py-2.5 rounded-lg shadow-lg text-sm font-medium
            transition-all duration-300
            ${toast.type === 'error'
              ? 'bg-red-500/90 text-white border border-red-400/30'
              : 'bg-green-500/90 text-white border border-green-400/30'
            }
          `}
        >
          {toast.msg}
        </div>
      )}
    </>
  );
}
