import { listBlockedDids } from './api';

let blocked = $state(new Set<string>());

export function getBlockedDids(): Set<string> {
  return blocked;
}

export function isBlocked(did: string): boolean {
  return blocked.has(did);
}

export function addBlocked(did: string) {
  blocked = new Set([...blocked, did]);
}

export function removeBlocked(did: string) {
  const next = new Set(blocked);
  next.delete(did);
  blocked = next;
}

export async function loadBlocklist(): Promise<void> {
  try {
    const dids = await listBlockedDids();
    blocked = new Set(dids);
  } catch { /* not logged in */ }
}

export function clearBlocklist() {
  blocked = new Set();
}
