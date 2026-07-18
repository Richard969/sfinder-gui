import { useEffect } from 'react';
import { useAppStore } from '@/stores/appStore';
import { useCommandStore } from '@/stores/commandStore';
import { useSfinderCommand } from '@/hooks/useSfinderCommand';
import { useEditorFumen } from '@/components/fumen/FumenEditorEmbed';
import { useFumenStore } from '@/stores/fumenStore';
import { usePageStore } from '@/stores/pageStore';
import FumenEditorEmbed from '@/components/fumen/FumenEditorEmbed';
import PatternInput from '@/components/forms/PatternInput';
import SpinOptions from '@/components/forms/SpinOptions';
import CommandRunner from '@/components/forms/CommandRunner';
import OutputViewer from '@/components/output/OutputViewer';

export default function SpinPage() {
  const jarInfo = useAppStore((s) => s.sfinderJarInfo);
  const javaInfo = useAppStore((s) => s.javaInfo);
  const status = useCommandStore((s) => s.status);
  const clearStatus = useCommandStore((s) => s.clearStatus);
  const execute = useSfinderCommand();
  useEffect(() => { clearStatus(); }, [clearStatus]);
  const editorFumen = useEditorFumen();
  const patterns = useFumenStore((s) => s.patterns);
  const setPatterns = useFumenStore((s) => s.setPatterns);
  const page = usePageStore((s) => s.spin);
  const update = usePageStore((s) => s.update);
  const reset = usePageStore((s) => s.reset);
  const clearedAt = useFumenStore((s) => s.clearedAt);
  useEffect(() => { if (clearedAt) reset('spin'); }, [clearedAt]);
  const showRare = useAppStore((s) => s.settings.showRareOptions);
  useEffect(() => { if (!showRare) update('spin', { filter: 'strict', roof: true, maxRoof: -1 }); }, [showRare]);
  const ready = javaInfo.installed && jarInfo.found;
  return (
    <div className="max-w-5xl mx-auto space-y-4">
      <FumenEditorEmbed visibleRows={page.rows} onVisibleRowsChange={(v) => update('spin', { rows: v })} />
      <PatternInput value={patterns} onChange={setPatterns} />
      <SpinOptions
        fillBottom={page.fillBottom} onFillBottomChange={(v) => update('spin', { fillBottom: v })}
        fillTop={page.fillTop} onFillTopChange={(v) => update('spin', { fillTop: v })}
        marginHeight={page.marginHeight} onMarginHeightChange={(v) => update('spin', { marginHeight: v })}
        line={page.line} onLineChange={(v) => update('spin', { line: v })}
        roof={page.roof} onRoofChange={(v) => update('spin', { roof: v })}
        maxRoof={page.maxRoof} onMaxRoofChange={(v) => update('spin', { maxRoof: v })}
        filter={page.filter} onFilterChange={(v) => update('spin', { filter: v })}
      />
      <CommandRunner status={status}
        onExecute={() => execute({
          command: 'spin', tetfu: [editorFumen], patterns,
          fillBottom: page.fillBottom, fillTop: page.fillTop,
          marginHeight: page.marginHeight, line: page.line,
          roof: page.roof, maxRoof: page.maxRoof, filter: page.filter,
        })}
        onCancel={() => {}} disabled={!ready || !patterns} />
      {status.type === 'success' && <OutputViewer output={status.output} command="spin" />}
    </div>
  );
}
