import { useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useAppStore } from '@/stores/appStore';
import type { JavaInfo } from '@/types/app';

export default function SettingsPage() {
  const settings = useAppStore((s) => s.settings);
  const updateSettings = useAppStore((s) => s.updateSettings);
  const javaInfo = useAppStore((s) => s.javaInfo);
  const setJavaInfo = useAppStore((s) => s.setJavaInfo);
  const sfinderJarInfo = useAppStore((s) => s.sfinderJarInfo);
  const setSfinderJarInfo = useAppStore((s) => s.setSfinderJarInfo);

  // Re-check Java when path changes
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const info = await invoke<JavaInfo>('check_java', {
          javaPath: settings.javaPath || null,
        });
        if (!cancelled) setJavaInfo(info);
      } catch { if (!cancelled) setJavaInfo({ installed: false }); }
    })();
    return () => { cancelled = true; };
  }, [settings.javaPath, setJavaInfo]);

  // Re-check JAR when path changes
  useEffect(() => {
    if (!settings.sfinderJarPath) return;
    let cancelled = false;
    (async () => {
      try {
        const info = await invoke<{ found: boolean; path?: string; version?: string }>(
          'check_sfinder_jar', { path: settings.sfinderJarPath }
        );
        if (!cancelled) setSfinderJarInfo(info);
      } catch { if (!cancelled) setSfinderJarInfo({ found: false }); }
    })();
    return () => { cancelled = true; };
  }, [settings.sfinderJarPath, setSfinderJarInfo]);

  const browseFile = useCallback(async (setter: (v: string) => void, filters?: { name: string; extensions: string[] }[]) => {
    const result = await open({ multiple: false, filters });
    if (result) setter(result as string);
  }, []);

  const browseDir = useCallback(async (setter: (v: string) => void) => {
    const result = await open({ directory: true, multiple: false });
    if (result) setter(result as string);
  }, []);

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold">Settings</h2>
        <p className="text-sm text-muted-foreground">Configure Java runtime and sfinder.jar paths.</p>
      </div>

      {/* Java */}
      <div className="rounded-lg border border-border bg-card">
        <div className="border-b border-border px-5 py-3"><h3 className="font-medium text-sm">Java Runtime</h3></div>
        <div className="p-5 space-y-3">
          <div className="flex gap-2">
            <input type="text" value={settings.javaPath}
              onChange={(e) => updateSettings({ javaPath: e.target.value })}
              placeholder="java (use system PATH)"
              className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm
                placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring" />
            <button onClick={() => browseFile((v) => updateSettings({ javaPath: v }),
              [{ name: 'Java', extensions: ['exe', ''] }])}
              className="px-3 py-2 rounded-md bg-secondary text-xs text-muted-foreground hover:bg-secondary/80 hover:text-foreground transition-colors shrink-0">
              Browse
            </button>
          </div>
          <div className="flex items-center gap-2 text-xs">
            <span className={`h-2 w-2 rounded-full ${javaInfo.installed ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-muted-foreground">
              {javaInfo.installed ? `Detected: ${javaInfo.version ?? 'OK'}` : 'Java not found — install JDK 17+'}
            </span>
          </div>
        </div>
      </div>

      {/* sfinder.jar */}
      <div className="rounded-lg border border-border bg-card">
        <div className="border-b border-border px-5 py-3"><h3 className="font-medium text-sm">sfinder.jar</h3></div>
        <div className="p-5 space-y-3">
          <div className="flex gap-2">
            <input type="text" value={settings.sfinderJarPath}
              onChange={(e) => updateSettings({ sfinderJarPath: e.target.value })}
              placeholder="C:\path\to\sfinder.jar"
              className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm
                placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring" />
            <button onClick={() => browseFile((v) => updateSettings({ sfinderJarPath: v }), [{ name: 'JAR', extensions: ['jar'] }])}
              className="px-3 py-2 rounded-md bg-secondary text-xs text-muted-foreground hover:bg-secondary/80 hover:text-foreground transition-colors shrink-0">
              Browse
            </button>
          </div>
          <div className="flex items-center gap-2 text-xs">
            <span className={`h-2 w-2 rounded-full ${sfinderJarInfo.found ? 'bg-green-500' : 'bg-yellow-500'}`} />
            <span className="text-muted-foreground">
              {sfinderJarInfo.found ? (sfinderJarInfo.version ?? 'JAR found') : 'Not found'}
            </span>
          </div>
        </div>
      </div>

      {/* Output */}
      <div className="rounded-lg border border-border bg-card">
        <div className="border-b border-border px-5 py-3"><h3 className="font-medium text-sm">Output</h3></div>
        <div className="p-5 space-y-3">
          <div className="flex gap-2">
            <input type="text" value={settings.outputDirectory}
              onChange={(e) => updateSettings({ outputDirectory: e.target.value })}
              placeholder="Default: output/"
              className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm
                placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring" />
            <button onClick={() => browseDir((v) => updateSettings({ outputDirectory: v }))}
              className="px-3 py-2 rounded-md bg-secondary text-xs text-muted-foreground hover:bg-secondary/80 hover:text-foreground transition-colors shrink-0">
              Browse
            </button>
          </div>
        </div>
      </div>

      {/* Theme + Language */}
      <div className="rounded-lg border border-border bg-card">
        <div className="border-b border-border px-5 py-3"><h3 className="font-medium text-sm">Appearance</h3></div>
        <div className="p-5 space-y-3">
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Theme</label>
            <select value={settings.theme}
              onChange={(e) => updateSettings({ theme: e.target.value as 'light' | 'dark' | 'system' })}
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm
                focus:outline-none focus:ring-2 focus:ring-ring">
              <option value="dark">Dark</option>
              <option value="light">Light</option>
              <option value="system">System</option>
            </select>
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Language</label>
            <select value={settings.language}
              onChange={(e) => updateSettings({ language: e.target.value as 'en' | 'zh' })}
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm
                focus:outline-none focus:ring-2 focus:ring-ring">
              <option value="en">English</option>
              <option value="zh">中文</option>
            </select>
          </div>
        </div>
      </div>
    </div>
  );
}
