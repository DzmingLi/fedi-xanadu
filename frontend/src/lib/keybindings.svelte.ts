import { getKeybindings, setKeybindings } from './api';
import { getToken } from './auth.svelte';
import type { Locale } from './i18n/index.svelte';

export interface KeyAction {
  id: string;
  labels: Record<Locale, string>;
  category: string;
  defaultKey: string;
}

// All available actions with default keybindings
export const ACTIONS: KeyAction[] = [
  // Navigation
  { id: 'goto.home', labels: { en: 'Go to Home', zh: '前往首页', fr: 'Aller à l\'accueil', de: 'Zur Startseite' }, category: 'navigation', defaultKey: 'g h' },
  { id: 'goto.skills', labels: { en: 'Go to Skills', zh: '前往技能', fr: 'Aller aux compétences', de: 'Zu den Fähigkeiten' }, category: 'navigation', defaultKey: 'g s' },
  { id: 'goto.library', labels: { en: 'Go to Library', zh: '前往书架', fr: 'Aller à la bibliothèque', de: 'Zur Bibliothek' }, category: 'navigation', defaultKey: 'g l' },
  { id: 'goto.about', labels: { en: 'Go to About', zh: '前往关于', fr: 'Aller à À propos', de: 'Zu „Über“' }, category: 'navigation', defaultKey: 'g a' },
  { id: 'goto.newArticle', labels: { en: 'New Article', zh: '新建文章', fr: 'Nouvel article', de: 'Neuer Artikel' }, category: 'navigation', defaultKey: 'n a' },
  { id: 'goto.newSeries', labels: { en: 'New Series', zh: '新建系列', fr: 'Nouvelle série', de: 'Neue Serie' }, category: 'navigation', defaultKey: 'n s' },

  // Global
  { id: 'search', labels: { en: 'Search', zh: '搜索', fr: 'Rechercher', de: 'Suchen' }, category: 'global', defaultKey: '/' },
  { id: 'help', labels: { en: 'Show shortcuts', zh: '显示快捷键', fr: 'Afficher les raccourcis', de: 'Tastaturkürzel anzeigen' }, category: 'global', defaultKey: '?' },
  { id: 'settings', labels: { en: 'Shortcut settings', zh: '快捷键设置', fr: 'Paramètres des raccourcis', de: 'Tastaturkürzel-Einstellungen' }, category: 'global', defaultKey: 'Ctrl+,' },

  // Article page
  { id: 'article.upvote', labels: { en: 'Upvote', zh: '赞', fr: 'Pour', de: 'Dafür' }, category: 'article', defaultKey: 'Shift+u' },
  { id: 'article.downvote', labels: { en: 'Downvote', zh: '踩', fr: 'Contre', de: 'Dagegen' }, category: 'article', defaultKey: 'Shift+d' },
  { id: 'article.bookmark', labels: { en: 'Bookmark', zh: '收藏', fr: 'Favori', de: 'Lesezeichen' }, category: 'article', defaultKey: 'b' },

  // List navigation
  { id: 'list.next', labels: { en: 'Next item', zh: '下一项', fr: 'Suivant', de: 'Nächstes' }, category: 'list', defaultKey: 'j' },
  { id: 'list.prev', labels: { en: 'Previous item', zh: '上一项', fr: 'Précédent', de: 'Vorheriges' }, category: 'list', defaultKey: 'k' },
  { id: 'list.open', labels: { en: 'Open item', zh: '打开', fr: 'Ouvrir', de: 'Öffnen' }, category: 'list', defaultKey: 'Enter' },
];

const STORAGE_KEY = 'fx_keybindings';

// State
let storedBindings: Record<string, string> = {};
const raw = localStorage.getItem(STORAGE_KEY);
if (raw) {
  try { storedBindings = JSON.parse(raw); } catch { /* ignore */ }
}

let userBindings = $state<Record<string, string>>(storedBindings);

export function getBinding(actionId: string): string {
  return userBindings[actionId] ?? ACTIONS.find(a => a.id === actionId)?.defaultKey ?? '';
}

export function getAllBindings(): Record<string, string> {
  const result: Record<string, string> = {};
  for (const action of ACTIONS) {
    result[action.id] = userBindings[action.id] ?? action.defaultKey;
  }
  return result;
}

export function getUserOverrides(): Record<string, string> {
  return { ...userBindings };
}

export function setBinding(actionId: string, key: string) {
  const next = { ...userBindings };
  if (key === ACTIONS.find(a => a.id === actionId)?.defaultKey) {
    delete next[actionId];
  } else {
    next[actionId] = key;
  }
  userBindings = next;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(userBindings));
}

export function resetBindings() {
  userBindings = {};
  localStorage.removeItem(STORAGE_KEY);
}

// Sync with server
export async function loadFromServer() {
  if (!getToken()) return;
  try {
    const data = await getKeybindings();
    if (data.bindings && Object.keys(data.bindings).length > 0) {
      userBindings = data.bindings;
      localStorage.setItem(STORAGE_KEY, JSON.stringify(userBindings));
    }
  } catch { /* not logged in or server error */ }
}

export async function saveToServer() {
  if (!getToken()) return;
  try {
    await setKeybindings(userBindings);
  } catch { /* ignore */ }
}

// Key matching
export function parseKeyCombo(combo: string): string[][] {
  return combo.split(' ').map(part => part.toLowerCase().split('+'));
}

export function matchesKey(event: KeyboardEvent, parts: string[]): boolean {
  const key = event.key.toLowerCase();
  const needCtrl = parts.includes('ctrl');
  const needShift = parts.includes('shift');
  const needAlt = parts.includes('alt');
  const needMeta = parts.includes('meta');

  if (event.ctrlKey !== needCtrl) return false;
  if (event.shiftKey !== needShift) return false;
  if (event.altKey !== needAlt) return false;
  if (event.metaKey !== needMeta) return false;

  const mainKey = parts.filter(p => !['ctrl', 'shift', 'alt', 'meta'].includes(p))[0];
  if (!mainKey) return false;

  if (mainKey === 'enter') return key === 'enter';
  if (mainKey === 'escape') return key === 'escape';
  if (mainKey === ',') return key === ',';
  if (mainKey === '/') return key === '/';
  if (mainKey === '?') return key === '?';

  return key === mainKey;
}

export function formatKeyDisplay(combo: string): string {
  return combo.split(' ').map(part => {
    return part.split('+').map(k => {
      const map: Record<string, string> = {
        'ctrl': 'Ctrl', 'shift': 'Shift', 'alt': 'Alt', 'meta': 'Cmd',
        'enter': 'Enter', 'escape': 'Esc',
      };
      return map[k.toLowerCase()] ?? k.toUpperCase();
    }).join('+');
  }).join(' ');
}

export const CATEGORY_LABELS: Record<string, Record<Locale, string>> = {
  navigation: { en: 'Navigation', zh: '导航', fr: 'Navigation', de: 'Navigation' },
  global: { en: 'Global', zh: '全局', fr: 'Global', de: 'Global' },
  article: { en: 'Article', zh: '文章', fr: 'Article', de: 'Artikel' },
  list: { en: 'List', zh: '列表', fr: 'Liste', de: 'Liste' },
};
