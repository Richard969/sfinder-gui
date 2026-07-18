import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { AppSettings, JavaInfo, SfinderJarInfo } from '@/types/app';

interface AppStore {
  settings: AppSettings;
  updateSettings: (partial: Partial<AppSettings>) => void;

  javaInfo: JavaInfo;
  setJavaInfo: (info: JavaInfo) => void;

  sfinderJarInfo: SfinderJarInfo;
  setSfinderJarInfo: (info: SfinderJarInfo) => void;
}

export const useAppStore = create<AppStore>()(
  persist(
    (set) => ({
      settings: {
        javaPath: '',
        sfinderJarPath: '',
        theme: 'dark',
        language: 'en',
        outputDirectory: '',
        showRareOptions: false,
      },
      javaInfo: { installed: false },
      sfinderJarInfo: { found: false },

      updateSettings: (partial) =>
        set((s) => ({ settings: { ...s.settings, ...partial } })),

      setJavaInfo: (info) => set({ javaInfo: info }),
      setSfinderJarInfo: (info) => set({ sfinderJarInfo: info }),
    }),
    {
      name: 'sfinder-gui-settings',
      partialize: (state) => ({ settings: state.settings }),
    },
  ),
);
