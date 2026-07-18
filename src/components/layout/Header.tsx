import { useState } from 'react';
import { useLocation } from 'react-router-dom';
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
            className="text-xs text-muted-foreground hover:text-foreground transition-colors px-3 py-1 rounded-md border border-border hover:border-muted-foreground/40 shrink-0">
            Usage
          </button>
          {showUsage && (
            <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={() => setShowUsage(false)}>
              <div className="w-full max-w-md rounded-xl border border-border bg-card p-6 shadow-2xl" onClick={(e) => e.stopPropagation()}>
                <h2 className="text-base font-semibold mb-3 text-foreground">{title}</h2>
                <div className="p-3">
                  <pre className="text-sm text-foreground leading-relaxed whitespace-pre-wrap font-sans">{usage}</pre>
                </div>
                <button onClick={() => setShowUsage(false)}
                  className="mt-4 w-full rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors">
                  Close
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </header>
  );
}
