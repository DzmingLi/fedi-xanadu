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

// Load-time completeness check: every locale must have every key.
// Fails the dev build early (rather than silently falling back to English at
// runtime when a key is missing). Uses zh as the canonical key set because
// TranslationKey = keyof typeof zh.
const CANONICAL_KEYS = Object.keys(zh);
if (import.meta.env.DEV) {
  for (const [code, dict] of Object.entries(messages)) {
    const missing = CANONICAL_KEYS.filter(k => !(k in dict));
    if (missing.length > 0) {
      throw new Error(
        `i18n: locale "${code}" is missing ${missing.length} key(s): ${missing.slice(0, 10).join(', ')}${missing.length > 10 ? ', …' : ''}`
      );
    }
  }
}

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
 *   t('home.bookmarks', 5) -> "5 bookmarks"
 * Placeholders are `{0}`, `{1}`, etc.
 *
 * No cross-locale fallback: if a key is absent from the active locale we
 * surface it loudly (dev: throw; prod: return the key itself) rather than
 * silently falling back to English. Every locale file must be complete.
 */
export function t(key: string, ...args: (string | number)[]): string {
  const raw = messages[locale]?.[key];
  if (raw === undefined) {
    if (import.meta.env.DEV) {
      throw new Error(`i18n: missing key "${key}" in locale "${locale}"`);
    }
    return key;
  }
  if (args.length === 0) return raw;
  return raw.replace(/\{(\d+)\}/g, (_, i) => String(args[Number(i)] ?? ''));
}
