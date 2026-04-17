import zh from './zh';
import en from './en';
import fr from './fr';
import de from './de';

export type TranslationKey = keyof typeof zh;
export type Locale = 'zh' | 'en' | 'fr' | 'de';

const messages: Record<Locale, Record<string, string>> = { zh, en, fr, de };

export const LOCALES: { code: Locale; label: string }[] = [
  { code: 'zh', label: '中文' },
  { code: 'en', label: 'English' },
  { code: 'fr', label: 'Français' },
  { code: 'de', label: 'Deutsch' },
];

/** Language display names -- usable regardless of current locale. */
export const LANG_NAMES: Record<string, string> = {
  zh: '中文', en: 'English', ja: '日本語', ko: '한국어',
  fr: 'Français', de: 'Deutsch', es: 'Español', pt: 'Português',
};

const STORAGE_KEY = 'fx_locale';

let initial: Locale = (localStorage.getItem(STORAGE_KEY) as Locale) || 'zh';
if (!messages[initial]) initial = 'zh';

let locale = $state<Locale>(initial);

export function getLocale(): Locale {
  return locale;
}

export function setLocale(newLocale: Locale) {
  locale = newLocale;
  localStorage.setItem(STORAGE_KEY, newLocale);
  document.documentElement.lang = newLocale;
}

/**
 * Translate a key with optional positional interpolation.
 *   t('article.viewAllForks', 5) -> "View all 5 forks ->"
 * Placeholders are `{0}`, `{1}`, etc.
 */
export function t(key: string, ...args: (string | number)[]): string {
  const raw = messages[locale]?.[key] ?? messages['en']?.[key] ?? key;
  if (args.length === 0) return raw;
  return raw.replace(/\{(\d+)\}/g, (_, i) => String(args[Number(i)] ?? ''));
}
