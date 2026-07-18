import { useState } from 'react';
import { useLocation } from 'react-router-dom';
import { X } from 'lucide-react';
import { useT } from '@/i18n/useTranslation';

export default function Header() {
  const t = useT();
  const location = useLocation();
  const [showUsage, setShowUsage] = useState(false);

  const routeTitles: Record<string, string> = {
    '/': 'Dashboard',
    '/fumen-editor': t('sidebar.fumenEditor'),
    '/settings': t('sidebar.settings'),
    '/percent': t('sidebar.percent'),
    '/path': t('sidebar.path'),
    '/setup': t('sidebar.setup'),
    '/ren': t('sidebar.ren'),
    '/spin': t('sidebar.spin'),
    '/cover': t('sidebar.cover'),
  };
  const routeDesc: Record<string, string> = {
    '/percent': t('percent.desc'),
    '/path': t('path.desc'),
    '/setup': t('setup.desc'),
    '/ren': t('ren.desc'),
    '/spin': t('spin.desc'),
    '/cover': t('cover.desc'),
  };
  const routeUsage: Record<string, string> = {
    '/percent': t('percent.usage'),
    '/path': t('path.usage'),
    '/spin': t('spin.usage'),
  };

  const title = routeTitles[location.pathname] ?? 'sfinder-gui';
  const desc = routeDesc[location.pathname] ?? '';
  const usage = routeUsage[location.pathname] ?? '';

  return (
    <header className="flex h-14 shrink-0 items-center justify-between border-b border-border bg-card/50 px-6">
      <div className="flex items-center gap-3 min-w-0">
        <h1 className="text-lg font-semibold tracking-tight shrink-0">{title}</h1>
        {desc && <p className="text-sm text-muted-foreground/70">{desc}</p>}
      </div>
      {usage && (
        <>
          <button onClick={() => setShowUsage(true)}
            className="text-xs text-foreground hover:text-accent-foreground transition-colors px-3 py-1 rounded-md border border-foreground/20 hover:border-foreground/40 shrink-0">
            Usage
          </button>
          {showUsage && (
            <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30" onClick={() => setShowUsage(false)}>
              <div className="relative w-full max-w-xl rounded-lg border border-border bg-card p-5 shadow-lg animate-in fade-in zoom-in-95"
                onClick={(e) => e.stopPropagation()}>
                <button onClick={() => setShowUsage(false)}
                  className="absolute right-3 top-3 text-muted-foreground hover:text-foreground transition-colors">
                  <X className="h-4 w-4" />
                </button>
                <h2 className="text-base font-semibold mb-3 text-foreground pr-6">{title}</h2>
                <pre className="text-sm text-foreground leading-relaxed whitespace-pre-wrap font-sans">{usage}</pre>
              </div>
            </div>
          )}
        </>
      )}
    </header>
  );
}
