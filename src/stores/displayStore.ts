import { create } from 'zustand';

interface DisplayStore {
  rows: number;
  setRows: (n: number) => void;
}

export const useDisplayStore = create<DisplayStore>((set) => ({
  rows: 12,
  setRows: (rows) => set({ rows }),
}));
