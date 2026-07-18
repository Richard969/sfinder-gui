import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '@/stores/appStore';
import { useCommandStore } from '@/stores/commandStore';
import type { SfinderCommandConfig, SfinderOutput } from '@/types/sfinder';

export function useSfinderCommand() {
  const settings = useAppStore((s) => s.settings);
  const setRunning = useCommandStore((s) => s.setRunning);
  const setSuccess = useCommandStore((s) => s.setSuccess);
  const setError = useCommandStore((s) => s.setError);

  const execute = useCallback(
    async (config: SfinderCommandConfig) => {
      const fullConfig: SfinderCommandConfig = {
        ...config,
        jarPath: config.jarPath || settings.sfinderJarPath,
        javaPath: config.javaPath || settings.javaPath,
      };

      setRunning(fullConfig);

      try {
        const output = await invoke<SfinderOutput>('run_sfinder_command', {
          config: fullConfig,
        });
        setSuccess(output);
      } catch (err: any) {
        setError(String(err), err?.stderr);
      }
    },
    [settings, setRunning, setSuccess, setError],
  );

  const cancel = useCallback(async () => {
    try {
      await invoke('cancel_command');
    } catch {}
    useCommandStore.getState().setCancelled();
  }, []);

  return { execute, cancel };
}
