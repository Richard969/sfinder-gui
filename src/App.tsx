import { useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '@/stores/appStore';
import AppLayout from '@/components/layout/AppLayout';
import HomePage from '@/routes/HomePage';
import FumenEditorPage from '@/routes/FumenEditorPage';
import SettingsPage from '@/routes/SettingsPage';
import PercentPage from '@/routes/PercentPage';
import PathPage from '@/routes/PathPage';
import SetupPage from '@/routes/SetupPage';
import RenPage from '@/routes/RenPage';
import SpinPage from '@/routes/SpinPage';
import CoverPage from '@/routes/CoverPage';
import ViewFumenPage from '@/routes/ViewFumenPage';
import type { JavaInfo } from '@/types/app';

export default function App() {
  const setJavaInfo = useAppStore((s) => s.setJavaInfo);
  const setSfinderJarInfo = useAppStore((s) => s.setSfinderJarInfo);
  const updateSettings = useAppStore((s) => s.updateSettings);
  const settings = useAppStore((s) => s.settings);
  const { sfinderJarPath, javaPath } = settings;
  const theme = settings.theme;

  // Apply theme
  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove('light', 'dark');
    if (theme === 'light') {
      root.classList.add('light');
    } else if (theme === 'dark') {
      root.classList.add('dark');
    }
    // 'system' — CSS media query handles it (no class)
  }, [theme]);

  // Discover Java + bundled JAR on startup
  useEffect(() => {
    (async () => {
      try {
        const info = await invoke<JavaInfo>('check_java', { javaPath: javaPath || null });
        setJavaInfo(info);
      } catch { setJavaInfo({ installed: false }); }
      try {
        const bundled = await invoke<string | null>('get_bundled_jar');
        if (bundled && !sfinderJarPath) {
          updateSettings({ sfinderJarPath: bundled });
        }
      } catch {}
    })();
  }, []);

  // Check JAR validity when path changes
  useEffect(() => {
    if (!sfinderJarPath) { setSfinderJarInfo({ found: false }); return; }
    (async () => {
      try {
        const info = await invoke('check_sfinder_jar', { path: sfinderJarPath });
        setSfinderJarInfo(info as any);
      } catch { setSfinderJarInfo({ found: false }); }
    })();
  }, [sfinderJarPath, setSfinderJarInfo]);

  return (
    <Routes>
      <Route path="/view-fumen" element={<ViewFumenPage />} />
      <Route path="*" element={
        <AppLayout>
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/fumen-editor" element={<FumenEditorPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/percent" element={<PercentPage />} />
            <Route path="/path" element={<PathPage />} />
            <Route path="/setup" element={<SetupPage />} />
            <Route path="/ren" element={<RenPage />} />
            <Route path="/spin" element={<SpinPage />} />
            <Route path="/cover" element={<CoverPage />} />
          </Routes>
        </AppLayout>
      } />
    </Routes>
  );
}
