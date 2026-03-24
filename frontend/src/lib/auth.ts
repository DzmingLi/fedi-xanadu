import type { AuthUser } from './types';

const STORAGE_KEY = 'fx_auth';

let _user: AuthUser | null = null;
let _listeners: Array<() => void> = [];

// Load from localStorage on init
const stored = localStorage.getItem(STORAGE_KEY);
if (stored) {
  try { _user = JSON.parse(stored); } catch { /* ignore */ }
}

export function getAuth(): AuthUser | null {
  return _user;
}

export function getToken(): string | null {
  return _user?.token ?? null;
}

export function setAuth(user: AuthUser | null) {
  _user = user;
  if (user) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(user));
  } else {
    localStorage.removeItem(STORAGE_KEY);
  }
  _listeners.forEach(fn => fn());
}

export function onAuthChange(fn: () => void): () => void {
  _listeners.push(fn);
  return () => { _listeners = _listeners.filter(f => f !== fn); };
}
