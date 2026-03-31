import type { AuthUser } from './types';

const STORAGE_KEY = 'fx_auth';

let stored: AuthUser | null = null;
const raw = localStorage.getItem(STORAGE_KEY);
if (raw) {
  try { stored = JSON.parse(raw); } catch { /* ignore */ }
}

let user = $state<AuthUser | null>(stored);

export function getAuth(): AuthUser | null {
  return user;
}

export function getToken(): string | null {
  return user?.token ?? null;
}

export function setAuth(newUser: AuthUser | null) {
  user = newUser;
  if (newUser) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(newUser));
  } else {
    localStorage.removeItem(STORAGE_KEY);
  }
}
