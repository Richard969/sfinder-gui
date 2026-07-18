import { HelpCircle } from 'lucide-react';
import { useT } from '@/i18n/useTranslation';

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
  filter: 'strict' | 'ignore-t' | 'none';
  onFilterChange: (v: 'strict' | 'ignore-t' | 'none') => void;
}

const FILTER_OPTIONS: { value: 'strict' | 'ignore-t' | 'none'; label: string }[] = [
  { value: 'strict', label: 'Strict' },
  { value: 'ignore-t', label: 'Ignore T' },
  { value: 'none', label: 'None' },
];

const HelpTooltip = ({ text }: { text: string }) => (
  <div className="group relative inline-flex">
    <HelpCircle className="h-3 w-3 text-muted-foreground/50 cursor-help" />
    <div className="absolute bottom-full left-0 mb-2 w-56 rounded-md border border-border bg-popover p-2.5
      text-xs text-popover-foreground shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible
      transition-all z-50 whitespace-pre-wrap leading-relaxed pointer-events-none">
      {text}
    </div>
  </div>
);

const NumInput = ({ label, value, onChange, min, hint, tooltip }: {
  label: string; value: number; onChange: (v: number) => void;
  min?: number; hint?: string; tooltip?: string;
}) => (
  <div className="space-y-1">
    <div className="flex items-center gap-1">
      <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">{label}</label>
      {tooltip && <HelpTooltip text={tooltip} />}
      {hint && <span className="ml-auto text-[9px] text-muted-foreground/60">{hint}</span>}
    </div>
    <input
      type="number"
      value={value}
      min={min}
      onChange={(e) => onChange(Number(e.target.value))}
      className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 font-mono text-sm
        placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-ring"
    />
  </div>
);

export default function SpinOptions(props: SpinOptionsProps) {
  const t = useT();
  const { fillBottom, onFillBottomChange, fillTop, onFillTopChange, marginHeight, onMarginHeightChange,
    line, onLineChange, roof, onRoofChange, maxRoof, onMaxRoofChange, filter, onFilterChange } = props;

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-1.5">
        <div className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
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
        <NumInput label={t('spin.line')} value={line} onChange={onLineChange}
          min={0} hint={t('spin.lineHint')} tooltip={t('spin.lineTooltip')} />
        <NumInput label={t('spin.maxRoof')} value={maxRoof} onChange={onMaxRoofChange}
          min={-1} hint={t('spin.maxRoofHint')} tooltip={t('spin.maxRoofTooltip')} />
        <div className="space-y-1">
          <div className="flex items-center gap-1">
            <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              {t('spin.roof')}
            </label>
            <HelpTooltip text={t('spin.roofTooltip')} />
          </div>
          <div className="flex gap-1">
            <button onClick={() => onRoofChange(true)}
              className={`flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors
                ${roof ? 'bg-primary text-primary-foreground' : 'bg-secondary text-muted-foreground hover:bg-secondary/80'}`}>
              {t('options.yes')}
            </button>
            <button onClick={() => onRoofChange(false)}
              className={`flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors
                ${!roof ? 'bg-primary text-primary-foreground' : 'bg-secondary text-muted-foreground hover:bg-secondary/80'}`}>
              {t('options.no')}
            </button>
          </div>
        </div>
      </div>
      <div className="space-y-1">
        <div className="flex items-center gap-1">
          <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
            {t('spin.filter')}
          </label>
          <HelpTooltip text={t('spin.filterTooltip')} />
        </div>
        <div className="flex gap-1">
          {FILTER_OPTIONS.map((opt) => (
            <button key={opt.value} onClick={() => onFilterChange(opt.value)}
              className={`flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors
                ${filter === opt.value
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-secondary text-muted-foreground hover:bg-secondary/80'}`}>
              {opt.label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
