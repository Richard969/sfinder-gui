import { useState, useEffect } from 'react';
import { useAppStore } from '@/stores/appStore';
import { useCommandStore } from '@/stores/commandStore';
import { useSfinderCommand } from '@/hooks/useSfinderCommand';
import { useEditorFumen } from '@/components/fumen/FumenEditorEmbed';
import { useFumenStore } from '@/stores/fumenStore';
import { useDisplayStore } from '@/stores/displayStore';
import { useT } from '@/i18n/useTranslation';
import FumenEditorEmbed from '@/components/fumen/FumenEditorEmbed';
import PatternInput from '@/components/forms/PatternInput';
import SpinOptions from '@/components/forms/SpinOptions';
import CommandRunner from '@/components/forms/CommandRunner';
import OutputViewer from '@/components/output/OutputViewer';
import type { FilterType } from '@/types/sfinder';

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
  const [fillBottom, setFillBottom] = useState(0);
  const [fillTop, setFillTop] = useState(-1);
  const [marginHeight, setMarginHeight] = useState(-1);
  const [line, setLine] = useState(2);
  const [roof, setRoof] = useState(true);
  const [maxRoof, setMaxRoof] = useState(-1);
  const [filter, setFilter] = useState<FilterType>('strict');
  const rows = useDisplayStore((s) => s.rows);
  const setRows = useDisplayStore((s) => s.setRows);
  const t = useT();
  const ready = javaInfo.installed && jarInfo.found;

  return (
    <div className="max-w-5xl mx-auto space-y-4">
      <FumenEditorEmbed visibleRows={rows} onVisibleRowsChange={setRows} />
      <PatternInput value={patterns} onChange={setPatterns} />
      <SpinOptions
        fillBottom={fillBottom} onFillBottomChange={setFillBottom}
        fillTop={fillTop} onFillTopChange={setFillTop}
        marginHeight={marginHeight} onMarginHeightChange={setMarginHeight}
        line={line} onLineChange={setLine}
        roof={roof} onRoofChange={setRoof}
        maxRoof={maxRoof} onMaxRoofChange={setMaxRoof}
        filter={filter} onFilterChange={setFilter}
      />
      <CommandRunner status={status}
        onExecute={() => execute({
          command: 'spin', tetfu: [editorFumen], patterns,
          fillBottom, fillTop, marginHeight, line, roof, maxRoof, filter,
        })}
        onCancel={() => {}} disabled={!ready || !editorFumen || !patterns} />
      {status.type === 'success' && <OutputViewer output={status.output} command="spin" />}
    </div>
  );
}
