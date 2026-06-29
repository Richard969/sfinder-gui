import { useEffect } from 'react';
import { useFumenStore } from '@/stores/fumenStore';
import type { CellType } from '@/types/fumen';

const PIECE_KEYS: Record<string, CellType> = {
  i: 'I', l: 'L', s: 'S', z: 'Z', t: 'T', o: 'O', j: 'J', g: 'X',
};

/** Keyboard shortcuts: I/L/S/Z/T/O/J/G switch brush color */
export function usePieceKeys() {
  const setTool = useFumenStore((s) => s.setTool);
  const tool = useFumenStore((s) => s.selectedTool);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      if (e.ctrlKey || e.altKey || e.metaKey) return;
      const piece = PIECE_KEYS[e.key.toLowerCase()];
      if (piece) {
        e.preventDefault();
        setTool({ type: 'paint', pieceType: piece, rotation: tool.type === 'paint' ? tool.rotation : 'spawn' });
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [setTool, tool]);
}
