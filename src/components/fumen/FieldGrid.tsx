import { useCallback, useRef } from 'react';
import { useFumenStore } from '@/stores/fumenStore';
import FieldCell from './FieldCell';
import type { CellType } from '@/types/fumen';
import { useT } from '@/i18n/useTranslation';

const COLS = 10;

interface FieldGridProps {
  visibleRows?: number;
  highlightedCells?: Set<string>;
}

export default function FieldGrid({ visibleRows, highlightedCells }: FieldGridProps) {
  const pages = useFumenStore((s) => s.pages);
  const currentPageIndex = useFumenStore((s) => s.currentPageIndex);
  const selectedTool = useFumenStore((s) => s.selectedTool);
  const setCell = useFumenStore((s) => s.setCell);
  const clearCell = useFumenStore((s) => s.clearCell);

  const page = pages[currentPageIndex];
  const field = page?.field;

  const isDragging = useRef(false);
  const isRightButton = useRef(false);

  const topY = visibleRows ? visibleRows - 1 : 22;

  const pieceType = selectedTool.type === 'paint' ? selectedTool.pieceType : 'X' as CellType;

  const doPaint = useCallback(
    (x: number, y: number, ctrlKey: boolean) => {
      if (ctrlKey) {
        for (let cx = 0; cx < COLS; cx++) {
          setCell(cx, y, pieceType);
        }
        setCell(x, y, '_');
      } else {
        setCell(x, y, pieceType);
      }
    },
    [pieceType, setCell],
  );

  const doErase = useCallback(
    (x: number, y: number, ctrlKey: boolean) => {
      if (ctrlKey) {
        for (let cx = 0; cx < COLS; cx++) clearCell(cx, y);
      } else {
        clearCell(x, y);
      }
    },
    [clearCell],
  );

  const handlePointerDown = useCallback(
    (x: number, y: number, e: React.PointerEvent) => {
      isDragging.current = true;
      isRightButton.current = e.button === 2;
      const ctrl = e.ctrlKey || e.metaKey;

      if (e.button === 2) {
        doErase(x, y, ctrl);
      } else {
        doPaint(x, y, ctrl);
      }
    },
    [doPaint, doErase],
  );

  const handlePointerEnter = useCallback(
    (x: number, y: number, e: React.PointerEvent) => {
      if (!isDragging.current || e.buttons === 0) return;
      const ctrl = e.ctrlKey || e.metaKey;

      if (isRightButton.current) {
        doErase(x, y, ctrl);
      } else {
        doPaint(x, y, ctrl);
      }
    },
    [doPaint, doErase],
  );

  const handlePointerUp = useCallback(() => {
    isDragging.current = false;
    isRightButton.current = false;
  }, []);

  const t = useT();

  if (!field) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        {t('editor.noFieldData')}
      </div>
    );
  }

  const rows: { y: number; cells: { x: number; type: CellType }[] }[] = [];
  for (let y = topY; y >= 0; y--) {
    const cells: { x: number; type: CellType }[] = [];
    for (let x = 0; x < COLS; x++) {
      const type = (field.at(x, y) ?? '_') as CellType;
      cells.push({ x, type });
    }
    rows.push({ y, cells });
  }

  return (
    <div
      className="inline-flex flex-col border border-border rounded-md overflow-hidden bg-background select-none"
      onPointerUp={handlePointerUp}
      onPointerLeave={handlePointerUp}
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className="flex bg-card border-b border-border">
        {Array.from({ length: COLS }, (_, x) => (
          <div
            key={x}
            className="flex items-center justify-center text-[10px] text-muted-foreground font-mono"
            style={{ width: 'var(--cell-size, 28px)', height: 16 }}
          >
            {x + 1}
          </div>
        ))}
      </div>

      <div className="flex flex-col">
        {rows.map((row) => (
          <div key={row.y} className="flex">
            {row.cells.map((cell) => (
              <FieldCell
                key={`${cell.x}-${row.y}`}
                type={cell.type}
                x={cell.x}
                y={row.y}
                onPointerDown={(e) => handlePointerDown(cell.x, row.y, e)}
                onPointerEnter={(e) => handlePointerEnter(cell.x, row.y, e)}
                isSelected={highlightedCells?.has(`${cell.x},${row.y}`) ?? false}
              />
            ))}
            <div
              className="flex items-center justify-center text-[10px] text-muted-foreground font-mono shrink-0"
              style={{ width: 20 }}
            >
              {row.y + 1}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
