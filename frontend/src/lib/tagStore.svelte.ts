import { listTags } from './api';
import { getLocale } from './i18n/index.svelte';
import type { Tag } from './types';

/**
 * Singleton-ish tag lookup used to render tag chips in the user's UI
 * locale. Every Tag row is its own identifier (no `name` / `names`
 * JSONB); localization happens by looking up the same group's sibling
 * in the target locale.
 *
 * Usage from a component:
 *   import { tagStore } from '../lib/tagStore.svelte';
 *   $effect(() => { tagStore.ensure(); });   // once per page
 *   ... {tagStore.localize('Math')}          // reactive on getLocale()
 */
class TagStore {
  #byId = $state(new Map<string, Tag>());
  #byGroup = $state(new Map<string, Map<string, string>>()); // group → lang → tag_id
  #loaded = $state(false);
  #loading = $state(false);

  /** Fetch the tag list once (no-op if already loaded/loading). */
  async ensure(limit = 500): Promise<void> {
    if (this.#loaded || this.#loading) return;
    this.#loading = true;
    try {
      const tags = await listTags(limit);
      const byId = new Map<string, Tag>();
      const byGroup = new Map<string, Map<string, string>>();
      for (const t of tags) {
        byId.set(t.id, t);
        if (t.group_id) {
          let g = byGroup.get(t.group_id);
          if (!g) {
            g = new Map();
            byGroup.set(t.group_id, g);
          }
          // First writer wins per lang — listTags is sorted by name, so
          // this is arbitrary; representative-aware lookup would require
          // an extra fetch but the per-lang map already narrows to one.
          if (!g.has(t.lang)) g.set(t.lang, t.id);
        }
      }
      this.#byId = byId;
      this.#byGroup = byGroup;
      this.#loaded = true;
    } finally {
      this.#loading = false;
    }
  }

  /** Force a re-fetch — call after creating/renaming a tag. */
  async refresh(limit = 500): Promise<void> {
    this.#loaded = false;
    await this.ensure(limit);
  }

  /**
   * Return the display string for a tag id in the current UI locale.
   * Resolution: find sibling matching UI locale → sibling in en →
   * the tag's own id (which already IS the display for its lang).
   */
  localize(id: string): string {
    const locale = getLocale();
    const tag = this.#byId.get(id);
    if (!tag) return id;
    if (tag.lang === locale) return tag.id;
    const group = this.#byGroup.get(tag.group_id);
    if (group) {
      const match = group.get(locale) ?? group.get('en');
      if (match) return match;
    }
    return tag.id;
  }

  /** Raw access for callers that need the full Tag record. */
  get(id: string): Tag | undefined {
    return this.#byId.get(id);
  }
}

export const tagStore = new TagStore();
