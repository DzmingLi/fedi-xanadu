import { getKeybindings, setKeybindings } from './api';
import { getToken } from './auth.svelte';

export interface KeyAction {
  id: string;
  label: string;
  labelZh: string;
  category: string;
  defaultKey: string;
}

// All available actions with default keybindings
export const ACTIONS: KeyAction[] = [
  // Navigation
  { id: 'goto.home', label: 'Go to Home', labelZh: '前往首页', category: 'navigation', defaultKey: 'g h' },
  { id: 'goto.skills', label: 'Go to Skills', labelZh: '前往技能', category: 'navigation', defaultKey: 'g s' },
  { id: 'goto.library', label: 'Go to Library', labelZh: '前往书架', category: 'navigation', defaultKey: 'g l' },
  { id: 'goto.about', label: 'Go to About', labelZh: '前往关于', category: 'navigation', defaultKey: 'g a' },
  { id: 'goto.newArticle', label: 'New Article', labelZh: '新建文章', category: 'navigation', defaultKey: 'n a' },
  { id: 'goto.newSeries', label: 'New Series', labelZh: '新建系列', category: 'navigation', defaultKey: 'n s' },

  // Global
  { id: 'search', label: 'Search', labelZh: '搜索', category: 'global', defaultKey: '/' },
  { id: 'help', label: 'Show shortcuts', labelZh: '显示快捷键', category: 'global', defaultKey: '?' },
  { id: 'settings', label: 'Shortcut settings', labelZh: '快捷键设置', category: 'global', defaultKey: 'Ctrl+,' },

  // Article page
  { id: 'article.upvote', label: 'Upvote', labelZh: '赞', category: 'article', defaultKey: 'Shift+u' },
  { id: 'article.downvote', label: 'Downvote', labelZh: '踩', category: 'article', defaultKey: 'Shift+d' },
  { id: 'article.bookmark', label: 'Bookmark', labelZh: '收藏', category: 'article', defaultKey: 'b' },

  // List navigation
  { id: 'list.next', label: 'Next item', labelZh: '下一项', category: 'list', defaultKey: 'j' },
  { id: 'list.prev', label: 'Previous item', labelZh: '上一项', category: 'list', defaultKey: 'k' },
  { id: 'list.open', label: 'Open item', labelZh: '打开', category: 'list', defaultKey: 'Enter' },
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

export const CATEGORY_LABELS: Record<string, { en: string; zh: string }> = {
  navigation: { en: 'Navigation', zh: '导航' },
  global: { en: 'Global', zh: '全局' },
  article: { en: 'Article', zh: '文章' },
  list: { en: 'List', zh: '列表' },
};

// Compatibility shim -- no-op, Svelte 5 reactivity handles this.
export function onBindingsChange(_fn: () => void): () => void {
  return () => {};
}
