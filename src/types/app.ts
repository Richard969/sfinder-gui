export interface JavaInfo {
  installed: boolean;
  version?: string;
  path?: string;
}

export interface SfinderJarInfo {
  found: boolean;
  path?: string;
  version?: string;
}

export type Theme = 'light' | 'dark' | 'system';
export type Language = 'en' | 'zh';

export interface AppSettings {
  javaPath: string;
  sfinderJarPath: string;
  theme: Theme;
  language: Language;
  outputDirectory: string;
  showRareOptions: boolean;
}
