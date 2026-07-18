import { useEffect, useMemo } from 'react';
import { useFumenStore } from '@/stores/fumenStore';
import FieldGrid from '@/components/fumen/FieldGrid';
import FumenToolbar from '@/components/fumen/FumenToolbar';
import PageNavigator from '@/components/fumen/PageNavigator';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { useT } from '@/i18n/useTranslation';
import { Mino } from 'tetris-fumen';

export default function ViewFumenPage() {
  const decodeFumen = useFumenStore((s) => s.decodeFumen);
  const pages = useFumenStore((s) => s.pages);
  const currentPageIndex = useFumenStore((s) => s.currentPageIndex);
  const goToPage = useFumenStore((s) => s.goToPage);
  // Read from sessionStorage (set by OutputViewer) or URL param
  const params = new URLSearchParams(window.location.search);
  const storageKey = params.get('key');
  const fumenStr = storageKey
    ? sessionStorage.getItem(storageKey) || ''
    : decodeURIComponent(
        window.location.search.replace(/^.*[?&]fumen=([^&]*).*$/, '$1')
      );

  useEffect(() => {
    if (fumenStr) decodeFumen(fumenStr);
  }, [fumenStr, decodeFumen]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'ArrowLeft') goToPage(Math.max(0, currentPageIndex - 1));
      if (e.key === 'ArrowRight') goToPage(Math.min(pages.length - 1, currentPageIndex + 1));
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [currentPageIndex, goToPage]);

  const t = useT();
  const total = pages.length;
  const currentPage = pages[currentPageIndex];
  const pageOperation = currentPage?.operation;
  const comment = currentPage?.comment;

  // Compute highlighted cells for the operation on this page
  const highlightedCells = useMemo(() => {
    if (!pageOperation) return new Set<string>();
    try {
      const m = new Mino(
        pageOperation.type as any,
        pageOperation.rotation as any,
        pageOperation.x,
        pageOperation.y,
      );
      return new Set(m.positions().map((p: { x: number; y: number }) => `${p.x},${p.y}`));
    } catch { return new Set<string>(); }
  }, [pageOperation]);
  const isAllSolutions = total > 5;

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-card shrink-0">
        <FumenToolbar hideScreenshot />
          <span className="text-muted-foreground">
            {isAllSolutions ? t('view.allSolutions') : t('view.solution')}
          </span>
          <span className="font-mono font-bold text-primary">
            {currentPageIndex + 1} / {total}
          </span>
        </div>

      {/* Main area */}
      <div className="flex-1 flex items-center justify-center p-4 min-h-0">
        <div className="flex items-center gap-4">
          {/* Previous page button */}
          <button
            onClick={() => goToPage(currentPageIndex - 1)}
            disabled={currentPageIndex <= 0}
            className="p-2 rounded-full hover:bg-secondary disabled:opacity-20 disabled:cursor-not-allowed transition-colors shrink-0"
          >
            <ChevronLeft className="h-6 w-6 text-muted-foreground" />
          </button>

          {/* Field grid + info */}
          <div className="flex flex-col items-center gap-2">
            <div className="border border-border rounded-lg overflow-hidden">
              <FieldGrid highlightedCells={highlightedCells} />
            </div>

            {/* Comment */}
            <div className="text-center h-6">
              {comment && (
                <span className="text-xs text-muted-foreground">{comment}</span>
              )}
            </div>
          </div>

          {/* Next page button */}
          <button
            onClick={() => goToPage(currentPageIndex + 1)}
            disabled={currentPageIndex >= total - 1}
            className="p-2 rounded-full hover:bg-secondary disabled:opacity-20 disabled:cursor-not-allowed transition-colors shrink-0"
          >
            <ChevronRight className="h-6 w-6 text-muted-foreground" />
          </button>
        </div>
      </div>

      {/* Bottom page strip */}
      <div className="flex items-center justify-center gap-4 px-4 py-3 border-t border-border bg-card shrink-0">
        <PageNavigator />
        {/* Page dots for quick navigation */}
        {isAllSolutions && (
          <div className="flex gap-0.5 flex-wrap justify-center max-w-md">
            {pages.map((_, i) => (
              <button
                key={i}
                onClick={() => goToPage(i)}
                className={`w-2 h-2 rounded-full transition-colors ${
                  i === currentPageIndex ? 'bg-primary' : 'bg-border hover:bg-muted-foreground'
                }`}
                title={`Page ${i + 1}`}
              />
            ))}
          </div>
        )}
      </div>

      {/* Keyboard navigation */}
      <div className="text-[10px] text-muted-foreground text-center py-1 shrink-0">
        {t('view.arrowKeys')}
      </div>
    </div>
  );
}
