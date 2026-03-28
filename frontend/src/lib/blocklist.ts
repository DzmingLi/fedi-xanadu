import { listBlockedDids } from './api';

let _blocked = new Set<string>();
let _listeners: Array<() => void> = [];

export function getBlockedDids(): Set<string> {
  return _blocked;
}

export function isBlocked(did: string): boolean {
  return _blocked.has(did);
}

export function addBlocked(did: string) {
  _blocked = new Set([..._blocked, did]);
  _listeners.forEach(fn => fn());
}

export function removeBlocked(did: string) {
  const next = new Set(_blocked);
  next.delete(did);
  _blocked = next;
  _listeners.forEach(fn => fn());
}

export function onBlocklistChange(fn: () => void): () => void {
  _listeners.push(fn);
  return () => { _listeners = _listeners.filter(f => f !== fn); };
}

export async function loadBlocklist(): Promise<void> {
  try {
    const dids = await listBlockedDids();
    _blocked = new Set(dids);
    _listeners.forEach(fn => fn());
  } catch { /* not logged in */ }
}

export function clearBlocklist() {
  _blocked = new Set();
  _listeners.forEach(fn => fn());
}
