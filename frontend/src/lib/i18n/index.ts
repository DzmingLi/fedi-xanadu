import zh from './zh';
import en from './en';
import fr from './fr';

export type TranslationKey = keyof typeof zh;
export type Locale = 'zh' | 'en' | 'fr';

const messages: Record<Locale, Record<string, string>> = { zh, en, fr };

export const LOCALES: { code: Locale; label: string }[] = [
  { code: 'zh', label: '中文' },
  { code: 'en', label: 'English' },
  { code: 'fr', label: 'Français' },
];

/** Language display names — usable regardless of current locale. */
export const LANG_NAMES: Record<string, string> = {
  zh: '中文', en: 'English', ja: '日本語', ko: '한국어',
  fr: 'Français', de: 'Deutsch', es: 'Español', pt: 'Português',
};

const STORAGE_KEY = 'fx_locale';

let _locale: Locale = (localStorage.getItem(STORAGE_KEY) as Locale) || 'zh';
if (!messages[_locale]) _locale = 'zh';

let _listeners: Array<() => void> = [];

export function getLocale(): Locale {
  return _locale;
}

export function setLocale(locale: Locale) {
  _locale = locale;
  localStorage.setItem(STORAGE_KEY, locale);
  document.documentElement.lang = locale;
  _listeners.forEach(fn => fn());
}

export function onLocaleChange(fn: () => void): () => void {
  _listeners.push(fn);
  return () => { _listeners = _listeners.filter(f => f !== fn); };
}

/**
 * Translate a key with optional positional interpolation.
 *   t('article.viewAllForks', 5) → "View all 5 forks →"
 * Placeholders are `{0}`, `{1}`, etc.
 */
export function t(key: string, ...args: (string | number)[]): string {
  const raw = messages[_locale]?.[key] ?? messages['en']?.[key] ?? key;
  if (args.length === 0) return raw;
  return raw.replace(/\{(\d+)\}/g, (_, i) => String(args[Number(i)] ?? ''));
}
