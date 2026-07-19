import { useCallback } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useT } from '@/i18n/useTranslation';
import type { HoldOption, DropType } from '@/types/sfinder';

interface CommandOptionsProps {
  hold: HoldOption;
  onHoldChange: (v: HoldOption) => void;
  drop: DropType;
  onDropChange: (v: DropType) => void;
  kicksPath: string;
  onKicksPathChange: (v: string) => void;
  split?: boolean;
  onSplitChange?: (v: boolean) => void;
}

const DROP_OPTIONS: { value: DropType; label: string }[] = [
  { value: 'softdrop', label: 'Softdrop' },
  { value: 'harddrop', label: 'Harddrop' },
  { value: '180', label: '180°' },
  { value: 't-softdrop', label: 'T-Softdrop' },
];

export default function CommandOptions({
  hold,
  onHoldChange,
  drop,
  onDropChange,
  kicksPath,
  onKicksPathChange,
  split,
  onSplitChange,
}: CommandOptionsProps) {
  const t = useT();
  return (
    <div className="space-y-3">
      <div className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
        {t('options.label')}
      </div>

      <div className="grid grid-cols-2 gap-3">
        {/* Hold */}
        <div className="space-y-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">{t('options.hold')}</label>
          <select
            value={hold}
            onChange={(e) => onHoldChange(e.target.value as HoldOption)}
            className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-sm
              focus:outline-none focus:ring-1 focus:ring-ring"
          >
            <option value="use">{t('options.use')}</option>
            <option value="avoid">{t('options.avoid')}</option>
          </select>
        </div>

        {/* Drop */}
        <div className="space-y-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">{t('options.drop')}</label>
          <select
            value={drop}
            onChange={(e) => onDropChange(e.target.value as DropType)}
            className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-sm
              focus:outline-none focus:ring-1 focus:ring-ring"
          >
            {DROP_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        {/* Kicks */}
        <div className="space-y-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">{t('options.kicksFile')}</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={kicksPath ? kicksPath.replace(/^.*[\\/]/, '') : ''}
              onChange={(e) => onKicksPathChange(e.target.value)}
              placeholder={t('options.srsDefault')}
              readOnly
              className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 text-sm text-muted-foreground
                cursor-default focus:outline-none"
            />
            <button
              onClick={async () => {
                const result = await open({ multiple: false, filters: [{ name: 'Kick files', extensions: ['properties', 'txt', 'kick'] }] });
                if (result) onKicksPathChange(result as string);
              }}
              className="px-3 py-1.5 rounded-md bg-secondary text-xs text-muted-foreground hover:bg-secondary/80 hover:text-foreground transition-colors shrink-0"
            >
              {t('options.browse')}
            </button>
            {kicksPath && kicksPath !== 'srs' && (
              <button onClick={() => onKicksPathChange('srs')}
                className="px-2 py-1.5 rounded-md text-xs text-red-400 hover:bg-red-500/15 transition-colors shrink-0">
                ✕
              </button>
            )}
          </div>
        </div>

        {/* Split (Path command) */}
        {onSplitChange && (
          <div className="space-y-1">
            <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              {t('options.split')} <span className="text-[10px] opacity-50">({t('options.splitDesc')})</span>
            </label>
            <select
              value={split ? 'yes' : 'no'}
              onChange={(e) => onSplitChange(e.target.value === 'yes')}
              className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-sm
                focus:outline-none focus:ring-1 focus:ring-ring"
            >
              <option value="no">{t('options.no')}</option>
              <option value="yes">{t('options.yes')}</option>
            </select>
          </div>
        )}
      </div>
    </div>
  );
}
