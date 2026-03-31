import type { UserSettings } from './types';
import { getSettings } from './api';
import { setLocale, getLocale } from './i18n/index.svelte';
import type { Locale } from './i18n/index.svelte';

const LOCALES_SET = new Set(['zh', 'en', 'fr']);

let settings = $state<UserSettings | null>(null);
let loaded = $state(false);

export function getLangPrefs(): UserSettings | null {
  return settings;
}

export function isLoaded(): boolean {
  return loaded;
}

export function setLangPrefs(s: UserSettings) {
  settings = s;
}

/**
 * Load user settings from server and sync UI locale to native_lang.
 * Call this on login.
 */
export async function loadLangPrefs(): Promise<UserSettings | null> {
  try {
    const s = await getSettings();
    settings = s;
    loaded = true;
    // Sync UI locale to native_lang if it's a supported locale
    if (LOCALES_SET.has(s.native_lang) && s.native_lang !== getLocale()) {
      setLocale(s.native_lang as Locale);
    }
    return s;
  } catch {
    loaded = true;
    return null;
  }
}

export function clearLangPrefs() {
  settings = null;
  loaded = false;
}
