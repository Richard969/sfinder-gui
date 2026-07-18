import { Play, Square, Loader2 } from 'lucide-react';
import { useT } from '@/i18n/useTranslation';
import type { CommandStatus } from '@/types/sfinder';

interface CommandRunnerProps {
  status: CommandStatus;
  onExecute: () => void;
  onCancel: () => void;
  disabled?: boolean;
}

export default function CommandRunner({
  status,
  onExecute,
  onCancel,
  disabled = false,
}: CommandRunnerProps) {
  const t = useT();
  const isRunning = status.type === 'running';

  return (
    <div className="flex items-center gap-3">
      {isRunning ? (
        <button onClick={onCancel}
          className="flex items-center gap-2 rounded-md bg-red-500/15 px-4 py-2 text-sm font-medium
            text-red-400 hover:bg-red-500/25 transition-colors">
          <Square className="h-3.5 w-3.5" />
          {t('runner.cancel')}
        </button>
      ) : (
        <button onClick={onExecute} disabled={disabled}
          className="flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium
            text-primary-foreground hover:bg-primary/90 transition-colors
            disabled:opacity-50 disabled:cursor-not-allowed">
          <Play className="h-3.5 w-3.5" />
          {t('runner.execute')}
        </button>
      )}
      {isRunning && (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          {t('runner.running')}
        </div>
      )}
      {status.type === 'success' && status.output.exitCode !== 0 && (
        <div className="text-xs text-red-400">{t('runner.error')}</div>
      )}
      {status.type === 'success' && status.output.exitCode === 0 && (
        <div className="text-xs text-green-400">{t('runner.completed')}</div>
      )}
      {status.type === 'error' && (
        <div className="text-xs text-red-400">{status.message}</div>
      )}
    </div>
  );
}
