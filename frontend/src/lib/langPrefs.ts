import type { UserSettings } from './types';
import { getSettings } from './api';
import { setLocale, getLocale } from './i18n';
import type { Locale } from './i18n';

let _settings: UserSettings | null = null;
let _listeners: Array<() => void> = [];
let _loaded = false;

const LOCALES_SET = new Set(['zh', 'en', 'fr']);

export function getLangPrefs(): UserSettings | null {
  return _settings;
}

export function isLoaded(): boolean {
  return _loaded;
}

export function setLangPrefs(s: UserSettings) {
  _settings = s;
  _listeners.forEach(fn => fn());
}

export function onLangPrefsChange(fn: () => void): () => void {
  _listeners.push(fn);
  return () => { _listeners = _listeners.filter(f => f !== fn); };
}

/**
 * Load user settings from server and sync UI locale to native_lang.
 * Call this on login.
 */
export async function loadLangPrefs(): Promise<UserSettings | null> {
  try {
    const s = await getSettings();
    _settings = s;
    _loaded = true;
    // Sync UI locale to native_lang if it's a supported locale
    if (LOCALES_SET.has(s.native_lang) && s.native_lang !== getLocale()) {
      setLocale(s.native_lang as Locale);
    }
    _listeners.forEach(fn => fn());
    return s;
  } catch {
    _loaded = true;
    return null;
  }
}

export function clearLangPrefs() {
  _settings = null;
  _loaded = false;
  _listeners.forEach(fn => fn());
}
