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

/// Check if we have an OAuth cookie session (for AT Protocol users).
/// Call this on app init — if an OAuth session exists but no localStorage token,
/// populate the auth state from the cookie-based session.
export async function checkOAuthSession() {
  if (user) return; // already logged in via localStorage
  try {
    const res = await fetch('/oauth/me', { credentials: 'same-origin' });
    if (res.ok) {
      const data = await res.json();
      // Create a synthetic AuthUser — token is empty (cookie-based)
      setAuth({
        token: '',
        did: data.did,
        handle: data.handle || '',
        display_name: null,
        avatar: null,
      });
    }
  } catch { /* no OAuth session */ }
}
