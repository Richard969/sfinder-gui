import { create } from 'zustand';

interface SpinPage {
  fillBottom: number;
  fillTop: number;
  marginHeight: number;
  line: number;
  roof: boolean;
  maxRoof: number;
  filter: 'strict' | 'ignore-t' | 'none';
  rows: number;
}

interface StandardPage {
  hold: 'use' | 'avoid';
  drop: 'softdrop' | 'harddrop' | '180' | 't-softdrop';
  kicks: string;
  split: boolean;
  clearLine: number;
  format: 'html' | 'csv';
}

interface CoverPage {
  hold: 'use' | 'avoid';
  drop: 'softdrop' | 'harddrop' | '180' | 't-softdrop';
  kicks: string;
  mode: string;
  coverLogic: 'or' | 'and';
  rows: number;
}

type PageName = 'standard' | 'spin' | 'cover';

interface PageStore {
  standard: StandardPage;
  spin: SpinPage;
  cover: CoverPage;
  update: (page: PageName, patch: Partial<StandardPage | SpinPage | CoverPage>) => void;
  reset: (page: PageName) => void;
}

const DEFAULTS = {
  standard: {
    hold: 'use' as const,
    drop: 'softdrop' as const,
    kicks: 'srs',
    split: false,
    clearLine: 4,
    format: 'csv' as const,
  },
  spin: {
    fillBottom: 0,
    fillTop: -1,
    marginHeight: -1,
    line: 2,
    roof: true,
    maxRoof: -1,
    filter: 'strict' as const,
    rows: 12,
  },
  cover: {
    hold: 'use' as const,
    drop: 'softdrop' as const,
    kicks: 'srs',
    mode: 'normal',
    coverLogic: 'or' as const,
    rows: 12,
  },
};

export const usePageStore = create<PageStore>((set) => ({
  standard: { ...DEFAULTS.standard },
  spin: { ...DEFAULTS.spin },
  cover: { ...DEFAULTS.cover },

  update: (page, patch) =>
    set((s) => ({ [page]: { ...s[page], ...patch } as any })),

  reset: (page) => set({ [page]: { ...DEFAULTS[page] } }),
}));
