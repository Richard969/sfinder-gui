import { create } from 'zustand';
import { decoder, encoder, Field } from 'tetris-fumen';
import type { Page as FumenPage, EncodePage } from 'tetris-fumen';
import type { CellType, PageFlags, FumenSnapshot, SelectedTool, RotationType } from '@/types/fumen';

const MAX_HISTORY = 50;
const EMPTY_FIELD_STR = '_'.repeat(10 * 23);
const EMPTY_GARBAGE_STR = '_'.repeat(10);

interface FieldPage {
  field: Field;
  comment: string;
  flags: PageFlags;
  operation?: { type: string; rotation: string; x: number; y: number };
}

function defaultFlags(): PageFlags {
  return { colorize: true, lock: false, mirror: false, quiz: false, rise: false };
}

function createEmptyFieldPage(): FieldPage {
  return {
    field: Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR),
    comment: '',
    flags: defaultFlags(),
    operation: undefined,
  };
}

// Deep-copy pages — use field.copy() for Field instances (structuredClone corrupts private fields)
function clonePages(pages: FieldPage[]): FieldPage[] {
  return pages.map((p) => ({
    field: p.field.copy(),
    comment: p.comment,
    flags: { ...p.flags },
    operation: p.operation ? { ...p.operation } : undefined,
  }));
}

function pagesToEncode(pages: FieldPage[]): EncodePage[] {
  return pages.map((p) => ({
    field: p.field,
    comment: p.comment || undefined,
    flags: { ...p.flags },
    operation: p.operation
      ? { type: p.operation.type as any, rotation: p.operation.rotation as any, x: p.operation.x, y: p.operation.y }
      : undefined,
  }));
}

function pagesFromFumen(fumenPages: FumenPage[]): FieldPage[] {
  return fumenPages.map((p) => ({
    field: p.field,
    comment: p.comment ?? '',
    flags: {
      colorize: p.flags?.colorize ?? true,
      lock: p.flags?.lock ?? false,
      mirror: p.flags?.mirror ?? false,
      quiz: p.flags?.quiz ?? false,
      rise: p.flags?.rise ?? false,
    },
    operation: p.operation
      ? { type: p.operation.type, rotation: p.operation.rotation, x: p.operation.x, y: p.operation.y }
      : undefined,
  }));
}

interface FumenStore {
  pages: FieldPage[];
  currentPageIndex: number;
  selectedTool: SelectedTool;
  fumenString: string;
  patterns: string;
  clearLine: number;
  spinFillBottom: number;
  spinFillTop: number;
  spinMarginHeight: number;
  spinLine: number;
  spinRoof: boolean;
  spinMaxRoof: number;
  spinFilter: 'strict' | 'ignore-t' | 'none';
  undoStack: FumenSnapshot[];
  redoStack: FumenSnapshot[];
  setPatterns: (patterns: string) => void;
  setClearLine: (n: number) => void;
  setSpinFillBottom: (n: number) => void;
  setSpinFillTop: (n: number) => void;
  setSpinMarginHeight: (n: number) => void;
  setSpinLine: (n: number) => void;
  setSpinRoof: (v: boolean) => void;
  setSpinMaxRoof: (n: number) => void;
  setSpinFilter: (v: 'strict' | 'ignore-t' | 'none') => void;
  newFile: () => void;
  decodeFumen: (str: string) => boolean;
  encodeFumen: () => string;
  setCell: (x: number, y: number, type: CellType) => void;
  clearCell: (x: number, y: number) => void;
  clearField: () => void;
  setTool: (tool: SelectedTool) => void;
  setComment: (comment: string) => void;
  setFlags: (flags: Partial<PageFlags>) => void;
  undo: () => void;
  redo: () => void;
  addPage: () => void;
  deletePage: () => boolean;
  goToPage: (index: number) => void;
  flipHorizontal: () => void;
  flipVertical: () => void;
  mirrorField: () => void;
}

function pushSnapshot(state: FumenStore): void {
  const snapshot: FumenSnapshot = {
    fumenString: encoder.encode(pagesToEncode(state.pages)),
    currentPageIndex: state.currentPageIndex,
  };
  state.undoStack.push(snapshot);
  if (state.undoStack.length > MAX_HISTORY) state.undoStack.shift();
  state.redoStack = [];
}

function restoreSnapshot(snapshot: FumenSnapshot): FieldPage[] {
  const fumenPages = decoder.decode(snapshot.fumenString);
  return pagesFromFumen(fumenPages);
}

function mutatePage(state: FumenStore): FieldPage[] {
  pushSnapshot(state);
  return clonePages(state.pages);
}

export const useFumenStore = create<FumenStore>((set, get) => ({
  pages: [createEmptyFieldPage()],
  currentPageIndex: 0,
  selectedTool: { type: 'paint', pieceType: 'X' as CellType, rotation: 'spawn' as RotationType },
  fumenString: '',
  patterns: '',
  clearLine: 4,
  spinFillBottom: 0,
  spinFillTop: -1,
  spinMarginHeight: -1,
  spinLine: 2,
  spinRoof: true,
  spinMaxRoof: -1,
  spinFilter: 'strict',
  undoStack: [],
  redoStack: [],
  setPatterns: (patterns) => set({ patterns }),
  setClearLine: (clearLine) => set({ clearLine }),
  setSpinFillBottom: (v) => set({ spinFillBottom: v }),
  setSpinFillTop: (v) => set({ spinFillTop: v }),
  setSpinMarginHeight: (v) => set({ spinMarginHeight: v }),
  setSpinLine: (v) => set({ spinLine: v }),
  setSpinRoof: (v) => set({ spinRoof: v }),
  setSpinMaxRoof: (v) => set({ spinMaxRoof: v }),
  setSpinFilter: (v) => set({ spinFilter: v }),
  newFile: () => set({
    pages: [createEmptyFieldPage()],
    currentPageIndex: 0,
    fumenString: '',
    patterns: '',
    undoStack: [],
    redoStack: [],
  }),

  decodeFumen: (str: string) => {
    try {
      let fumenStr = str.trim();
      const urlMatch = fumenStr.match(/\?(v115@.+)$/);
      if (urlMatch) fumenStr = urlMatch[1];
      if (!fumenStr.startsWith('v115@') && !fumenStr.startsWith('v110@')) {
        fumenStr = 'v115@' + fumenStr;
      }
      const fumenPages = decoder.decode(fumenStr);
      if (!fumenPages || fumenPages.length === 0) return false;
      set({
        pages: pagesFromFumen(fumenPages),
        currentPageIndex: 0,
        fumenString: fumenStr,
        undoStack: [],
        redoStack: [],
      });
      return true;
    } catch (e) {
      console.error('decodeFumen error:', e);
      return false;
    }
  },

  encodeFumen: () => encoder.encode(pagesToEncode(get().pages)),

  setCell: (x, y, type) => set((state) => {
    const newPages = mutatePage(state);
    const page = newPages[state.currentPageIndex];
    page.field.set(x, y, type);
    return { pages: newPages, fumenString: encoder.encode(pagesToEncode(newPages)) };
  }),

  clearCell: (x, y) => set((state) => {
    const newPages = mutatePage(state);
    newPages[state.currentPageIndex].field.set(x, y, '_');
    return { pages: newPages, fumenString: encoder.encode(pagesToEncode(newPages)) };
  }),

  clearField: () => set((state) => {
    pushSnapshot(state);
    const newPages = clonePages(state.pages);
    newPages[state.currentPageIndex] = createEmptyFieldPage();
    return { pages: newPages, fumenString: encoder.encode(pagesToEncode(newPages)) };
  }),

  setTool: (tool) => set({ selectedTool: tool }),

  setComment: (comment) => set((state) => {
    const newPages = clonePages(state.pages);
    newPages[state.currentPageIndex].comment = comment;
    return { pages: newPages };
  }),

  setFlags: (flags) => set((state) => {
    const newPages = clonePages(state.pages);
    Object.assign(newPages[state.currentPageIndex].flags, flags);
    return { pages: newPages };
  }),

  undo: () => {
    const { undoStack } = get();
    if (undoStack.length === 0) return;
    set((state) => {
      const current: FumenSnapshot = {
        fumenString: encoder.encode(pagesToEncode(state.pages)),
        currentPageIndex: state.currentPageIndex,
      };
      const prev = state.undoStack[state.undoStack.length - 1];
      return {
        pages: restoreSnapshot(prev),
        currentPageIndex: prev.currentPageIndex,
        fumenString: prev.fumenString,
        undoStack: state.undoStack.slice(0, -1),
        redoStack: [...state.redoStack, current],
      };
    });
  },

  redo: () => {
    const { redoStack } = get();
    if (redoStack.length === 0) return;
    set((state) => {
      const current: FumenSnapshot = {
        fumenString: encoder.encode(pagesToEncode(state.pages)),
        currentPageIndex: state.currentPageIndex,
      };
      const next = state.redoStack[state.redoStack.length - 1];
      return {
        pages: restoreSnapshot(next),
        currentPageIndex: next.currentPageIndex,
        fumenString: next.fumenString,
        undoStack: [...state.undoStack, current],
        redoStack: state.redoStack.slice(0, -1),
      };
    });
  },

  addPage: () => set((state) => {
    const newPages = clonePages(state.pages);
    newPages.push(createEmptyFieldPage());
    return {
      pages: newPages,
      currentPageIndex: newPages.length - 1,
      fumenString: encoder.encode(pagesToEncode(newPages)),
    };
  }),

  deletePage: () => {
    const { pages, currentPageIndex } = get();
    if (pages.length <= 1) return false;
    set((state) => {
      const newPages = clonePages(state.pages).filter((_, i) => i !== currentPageIndex);
      return {
        pages: newPages,
        currentPageIndex: Math.min(currentPageIndex, newPages.length - 1),
        fumenString: encoder.encode(pagesToEncode(newPages)),
      };
    });
    return true;
  },

  goToPage: (index) => {
    const { pages } = get();
    if (index >= 0 && index < pages.length) set({ currentPageIndex: index });
  },

  flipHorizontal: () => set((state) => {
    const newPages = mutatePage(state);
    const oldField = newPages[state.currentPageIndex].field;
    const newField = Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR);
    for (let y = -1; y <= 22; y++)
      for (let x = 0; x < 10; x++)
        newField.set(9 - x, y, oldField.at(x, y));
    newPages[state.currentPageIndex].field = newField;
    return { pages: newPages, fumenString: encoder.encode(pagesToEncode(newPages)) };
  }),

  flipVertical: () => set((state) => {
    const newPages = mutatePage(state);
    const oldField = newPages[state.currentPageIndex].field;
    // Find highest non-empty row
    let topY = 0;
    for (let y = 22; y >= 0; y--) {
      let empty = true;
      for (let x = 0; x < 10; x++) {
        if (oldField.at(x, y) !== '_') { empty = false; break; }
      }
      if (!empty) { topY = y; break; }
    }
    const newField = Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR);
    // Flip within [0, topY] range
    for (let y = 0; y <= topY; y++) {
      for (let x = 0; x < 10; x++) {
        newField.set(x, topY - y, oldField.at(x, y));
      }
    }
    newPages[state.currentPageIndex].field = newField;
    return { pages: newPages, fumenString: encoder.encode(pagesToEncode(newPages)) };
  }),

  mirrorField: () => set((state) => {
    const newPages = mutatePage(state);
    const oldField = newPages[state.currentPageIndex].field;
    const newField = Field.create(EMPTY_FIELD_STR, EMPTY_GARBAGE_STR);
    const mirrorMap: Record<string, string> = { L: 'J', J: 'L', S: 'Z', Z: 'S' };
    for (let y = -1; y <= 22; y++)
      for (let x = 0; x < 10; x++) {
        const cell = oldField.at(x, y);
        newField.set(9 - x, y, mirrorMap[cell] ?? cell);
      }
    newPages[state.currentPageIndex].field = newField;
    return { pages: newPages, fumenString: encoder.encode(pagesToEncode(newPages)) };
  }),
}));
