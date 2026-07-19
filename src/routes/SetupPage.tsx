import FumenEditorEmbed from '@/components/fumen/FumenEditorEmbed';
import { useT } from '@/i18n/useTranslation';

export default function SetupPage() {
  const t = useT();
  return (
    <div className="max-w-5xl mx-auto space-y-4">
      <FumenEditorEmbed visibleRows={4} />
      <div className="flex items-center justify-center py-20">
        <div className="text-center space-y-3">
          <div className="text-4xl">🚧</div>
          <h3 className="text-lg font-semibold text-muted-foreground">{t('wip.title')}</h3>
          <p className="text-sm text-muted-foreground max-w-md">{t('wip.desc')}</p>
        </div>
      </div>
    </div>
  );
}
