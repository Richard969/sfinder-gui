import { useState, useEffect } from 'react';
import { useAppStore } from '@/stores/appStore';
import { useCommandStore } from '@/stores/commandStore';
import { useSfinderCommand } from '@/hooks/useSfinderCommand';
import { useEditorFumen } from '@/components/fumen/FumenEditorEmbed';
import { useFumenStore } from '@/stores/fumenStore';
import { useT } from '@/i18n/useTranslation';
import FumenEditorEmbed from '@/components/fumen/FumenEditorEmbed';
import PatternInput from '@/components/forms/PatternInput';
import CommandOptions from '@/components/forms/CommandOptions';
import CommandRunner from '@/components/forms/CommandRunner';
import OutputViewer from '@/components/output/OutputViewer';
import type { HoldOption, DropType } from '@/types/sfinder';

export default function PercentPage() {
  const jarInfo = useAppStore((s) => s.sfinderJarInfo);
  const javaInfo = useAppStore((s) => s.javaInfo);
  const status = useCommandStore((s) => s.status);
  const clearStatus = useCommandStore((s) => s.clearStatus);
  const execute = useSfinderCommand();
  const editorFumen = useEditorFumen();
  useEffect(() => { clearStatus(); }, [clearStatus]);

  const patterns = useFumenStore((s) => s.patterns);
  const setPatterns = useFumenStore((s) => s.setPatterns);
  const currentPageIndex = useFumenStore((s) => s.currentPageIndex);
  const [hold, setHold] = useState<HoldOption>('use');
  const [drop, setDrop] = useState<DropType>('softdrop');
  const [kicksPath, setKicksPath] = useState('srs');
  const page = currentPageIndex + 1;
  const clearLine = useFumenStore((s) => s.clearLine);
  const setClearLine = useFumenStore((s) => s.setClearLine);
  const t = useT();
  const ready = javaInfo.installed && jarInfo.found;

  return (
    <div className="max-w-5xl mx-auto space-y-4">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold">{t('percent.title')}</h2>
        <p className="text-sm text-muted-foreground">{t('percent.desc')}</p>
      </div>

      <FumenEditorEmbed visibleRows={clearLine} onVisibleRowsChange={setClearLine} />
      <PatternInput value={patterns} onChange={setPatterns} />

      <CommandOptions
        hold={hold} onHoldChange={setHold} drop={drop} onDropChange={setDrop}
        kicksPath={kicksPath} onKicksPathChange={setKicksPath}
      />
      <CommandRunner status={status}
        onExecute={() => execute({ command: 'percent', tetfu: editorFumen, patterns, hold, drop, kicks: kicksPath, page, clearLine })}
        onCancel={() => {}} disabled={!ready || !editorFumen || !patterns} />
      {status.type === 'success' && <OutputViewer output={status.output} command="percent" />}
    </div>
  );
}
