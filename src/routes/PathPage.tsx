import { useEffect } from 'react';
import { useAppStore } from '@/stores/appStore';
import { useCommandStore } from '@/stores/commandStore';
import { useSfinderCommand } from '@/hooks/useSfinderCommand';
import { useEditorFumen } from '@/components/fumen/FumenEditorEmbed';
import { useFumenStore } from '@/stores/fumenStore';
import { usePageStore } from '@/stores/pageStore';
import { useT } from '@/i18n/useTranslation';
import FumenEditorEmbed from '@/components/fumen/FumenEditorEmbed';
import PatternInput from '@/components/forms/PatternInput';
import CommandOptions from '@/components/forms/CommandOptions';
import CommandRunner from '@/components/forms/CommandRunner';
import OutputViewer from '@/components/output/OutputViewer';

export default function PathPage() {
  const jarInfo = useAppStore((s) => s.sfinderJarInfo);
  const javaInfo = useAppStore((s) => s.javaInfo);
  const status = useCommandStore((s) => s.status);
  const clearStatus = useCommandStore((s) => s.clearStatus);
  const isRunning = status.type === 'running';
  const { execute, cancel } = useSfinderCommand();
  useEffect(() => {
    clearStatus();
    return () => { if (isRunning) cancel(); };
  }, [clearStatus, isRunning, cancel]);
  const editorFumen = useEditorFumen();
  const patterns = useFumenStore((s) => s.patterns);
  const setPatterns = useFumenStore((s) => s.setPatterns);
  const currentPageIndex = useFumenStore((s) => s.currentPageIndex);
  const std = usePageStore((s) => s.standard);
  const update = usePageStore((s) => s.update);
  const clearedAt = useFumenStore((s) => s.clearedAt);
  useEffect(() => { if (clearedAt) update('standard', { clearLine: 4 }); }, [clearedAt]);
  const page = currentPageIndex + 1;
  const clearLine = useFumenStore((s) => s.clearLine);
  const setClearLine = useFumenStore((s) => s.setClearLine);
  const showRare = useAppStore((s) => s.settings.showRareOptions);
  useEffect(() => { if (!showRare) update('standard', { split: false }); }, [showRare]);
  const ready = javaInfo.installed && jarInfo.found;
  const t = useT();

  return (
    <div className="max-w-5xl mx-auto space-y-4">
      <FumenEditorEmbed visibleRows={clearLine} onVisibleRowsChange={setClearLine} />
      <PatternInput value={patterns} onChange={setPatterns} />
      <CommandOptions hold={std.hold} onHoldChange={(v) => update('standard', { hold: v })}
        drop={std.drop} onDropChange={(v) => update('standard', { drop: v })}
        kicksPath={std.kicks} onKicksPathChange={(v) => update('standard', { kicks: v })}
        {...(showRare ? { split: std.split, onSplitChange: (v: boolean) => update('standard', { split: v }) } : {})} />
      <CommandRunner status={status}
        onExecute={() => execute({
          command: 'path',
          tetfu: editorFumen ? [editorFumen] : [],
          patterns, hold: std.hold, drop: std.drop, kicks: std.kicks, format: 'csv', key: 'pattern', page, clearLine, split: std.split,
        })}
        onCancel={() => cancel()} disabled={!ready || !patterns} />
      {status.type === 'success' && <OutputViewer output={status.output} command="path" />}
    </div>
  );
}
