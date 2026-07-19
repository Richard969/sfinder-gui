import { useState, useEffect } from 'react';
import { HelpCircle } from 'lucide-react';
import { useT } from '@/i18n/useTranslation';
import { useAppStore } from '@/stores/appStore';

interface SpinOptionsProps {
  fillBottom: number;
  onFillBottomChange: (v: number) => void;
  fillTop: number;
  onFillTopChange: (v: number) => void;
  marginHeight: number;
  onMarginHeightChange: (v: number) => void;
  line: number;
  onLineChange: (v: number) => void;
  roof: boolean;
  onRoofChange: (v: boolean) => void;
  maxRoof: number;
  onMaxRoofChange: (v: number) => void;
  filter?: 'strict' | 'ignore-t' | 'none';
  onFilterChange?: (v: 'strict' | 'ignore-t' | 'none') => void;
}

const HelpTooltip = ({ text }: { text: string }) => (
  <div className="group relative inline-flex">
    <HelpCircle className="h-3 w-3 text-muted-foreground/50 cursor-help" />
    <div className="absolute bottom-full left-0 mb-2 w-72 rounded-md border border-border bg-popover p-2.5
      text-xs text-popover-foreground shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible
      transition-all z-50 whitespace-pre-wrap leading-relaxed pointer-events-none">
      {text}
    </div>
  </div>
);

const NumInput = ({ label, value, onChange, min, hint, tooltip }: {
  label: string; value: number; onChange: (v: number) => void;
  min?: number; hint?: string; tooltip?: string;
}) => {
  const [local, setLocal] = useState(String(value));
  useEffect(() => { setLocal(String(value)); }, [value]);
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-1">
        <label className="text-xs text-muted-foreground">{label}</label>
        {tooltip && <HelpTooltip text={tooltip} />}
        {hint && <span className="ml-auto text-[9px] text-muted-foreground/60">{hint}</span>}
      </div>
      <input
        type="number"
        value={local}
        min={min}
        onChange={(e) => {
          const raw = e.target.value;
          setLocal(raw);
          const n = parseFloat(raw);
          if (!isNaN(n)) onChange(n);
        }}
        className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-sm
          focus:outline-none focus:ring-1 focus:ring-ring
          [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
      />
    </div>
  );
};

export default function SpinOptions(props: SpinOptionsProps) {
  const t = useT();
  const showRare = useAppStore((s) => s.settings.showRareOptions);
  const { fillBottom, onFillBottomChange, fillTop, onFillTopChange, marginHeight, onMarginHeightChange,
    line, onLineChange, roof, onRoofChange, maxRoof, onMaxRoofChange, filter,
    onFilterChange } = props;
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-1.5">
        <div className="text-xs text-muted-foreground">
          {t('spin.optionsLabel')}
        </div>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <NumInput label={t('spin.fillBottom')} value={fillBottom} onChange={onFillBottomChange}
          min={0} hint={t('spin.fillBottomHint')} tooltip={t('spin.fillBottomTooltip')} />
        <NumInput label={t('spin.fillTop')} value={fillTop} onChange={onFillTopChange}
          min={-1} hint={t('spin.fillTopHint')} tooltip={t('spin.fillTopTooltip')} />
        <NumInput label={t('spin.marginHeight')} value={marginHeight} onChange={onMarginHeightChange}
          min={-1} hint={t('spin.marginHeightHint')} tooltip={t('spin.marginHeightTooltip')} />
        <div className="space-y-1">
          <div className="flex items-center gap-1">
            <label className="text-xs text-muted-foreground">
              {t('spin.line')}
            </label>
            <HelpTooltip text={t('spin.lineTooltip')} />
          </div>
          <div className="flex gap-1">
            <button onClick={() => onLineChange(1)}
              className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors
                ${line === 1 ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}>
              ≥ TSS
            </button>
            <button onClick={() => onLineChange(2)}
              className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors
                ${line === 2 ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}>
              ≥ TSD
            </button>
            <button onClick={() => onLineChange(3)}
              className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors
                ${line === 3 ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}>
              ≥ TST
            </button>
          </div>
        </div>
      </div>
      {showRare && (
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <NumInput label={t('spin.maxRoof')} value={maxRoof} onChange={onMaxRoofChange}
              min={-1} hint={t('spin.maxRoofHint')} tooltip={t('spin.maxRoofTooltip')} />
            <div className="space-y-1">
              <div className="flex items-center gap-1">
                <label className="text-xs text-muted-foreground">
                  {t('spin.roof')}
                </label>
                <HelpTooltip text={t('spin.roofTooltip')} />
              </div>
              <div className="flex gap-1">
                <button onClick={() => onRoofChange(true)}
                  className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors
                    ${roof ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}>
                  {t('options.yes')}
                </button>
                <button onClick={() => onRoofChange(false)}
                  className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors
                    ${!roof ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}>
                  {t('options.no')}
                </button>
              </div>
            </div>
          </div>
          {filter !== undefined && onFilterChange && (
            <div className="space-y-1">
              <div className="flex items-center gap-1">
                <label className="text-xs text-muted-foreground">
                  {t('spin.filter')}
                </label>
                <HelpTooltip text={t('spin.filterTooltip')} />
              </div>
              <div className="flex gap-1">
                {(['strict', 'ignore-t', 'none'] as const).map((val) => (
                  <button key={val} onClick={() => onFilterChange(val)}
                    className={`flex-1 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors
                      ${filter === val ? 'bg-primary/15 text-primary' : 'bg-background text-muted-foreground hover:bg-secondary'}`}>
                    {{ strict: 'Strict', 'ignore-t': 'Ignore T', none: 'None' }[val]}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
