import { useFumenStore } from '@/stores/fumenStore';
import { usePieceKeys } from '@/hooks/usePieceKeys';
import FieldGrid from './FieldGrid';
import PiecePalette from './PiecePalette';
import FumenToolbar from './FumenToolbar';
import PageNavigator from './PageNavigator';
import { useT } from '@/i18n/useTranslation';

interface FumenEditorEmbedProps {
  visibleRows?: number;
  onVisibleRowsChange?: (rows: number) => void;
  className?: string;
}

export default function FumenEditorEmbed({
  visibleRows, onVisibleRowsChange, className,
}: FumenEditorEmbedProps) {
  const t = useT();
  usePieceKeys();

  return (
    <div className={`flex flex-col gap-2 ${className ?? ''}`}>
      {/* Header: toolbar + controls */}
      <div className="flex items-center gap-3">
        <FumenToolbar />

        <div className="flex items-center gap-3 ml-auto">
          {/* Page navigator */}
          <PageNavigator />

          {/* Clear Line */}
          {onVisibleRowsChange && visibleRows !== undefined && (
            <div className="flex items-center gap-0.5 text-xs text-muted-foreground">
              <span>{t('editor.rows')}:</span>
              <button
                onClick={() => onVisibleRowsChange(Math.max(1, visibleRows - 1))}
                className="h-6 w-5 rounded-l border border-input bg-background hover:bg-secondary text-muted-foreground flex items-center justify-center text-xs transition-colors"
              >−</button>
              <span className="h-6 w-8 border-y border-input bg-background flex items-center justify-center font-mono text-xs tabular-nums">
                {visibleRows}
              </span>
              <button
                onClick={() => onVisibleRowsChange(Math.min(23, visibleRows + 1))}
                className="h-6 w-5 rounded-r border border-input bg-background hover:bg-secondary text-muted-foreground flex items-center justify-center text-xs transition-colors"
              >+</button>
            </div>
          )}
        </div>
      </div>

      {/* Grid + Palette */}
      <div className="flex gap-3">
        <FieldGrid visibleRows={visibleRows} />
        <PiecePalette />
      </div>
    </div>
  );
}

export function useEditorFumen(): string {
  return useFumenStore((s) => s.fumenString);
}
