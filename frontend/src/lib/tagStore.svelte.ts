import { listTags } from './api';
import { getLocale } from './i18n/index.svelte';
import type { Tag } from './types';

/**
 * Singleton-ish label lookup used to render tag chips in the user's UI
 * locale. Each `Tag` row here is one *label* (per-language display);
 * labels sharing the same `tag_id` name the same concept. Localization
 * picks the sibling whose `lang` matches the UI locale.
 *
 * Usage from a component:
 *   import { tagStore } from '../lib/tagStore.svelte';
 *   $effect(() => { tagStore.ensure(); });   // once per page
 *   ... {tagStore.localize('Math')}          // reactive on getLocale()
 */
class TagStore {
  #byId = $state(new Map<string, Tag>());
  #byTag = $state(new Map<string, Map<string, string>>()); // tag_id → lang → label_id
  #loaded = $state(false);
  #loading = $state(false);

  /** Fetch the label list once (no-op if already loaded/loading). */
  async ensure(limit = 500): Promise<void> {
    if (this.#loaded || this.#loading) return;
    this.#loading = true;
    try {
      const tags = await listTags(limit);
      const byId = new Map<string, Tag>();
      const byTag = new Map<string, Map<string, string>>();
      for (const t of tags) {
        byId.set(t.id, t);
        if (t.tag_id) {
          let g = byTag.get(t.tag_id);
          if (!g) {
            g = new Map();
            byTag.set(t.tag_id, g);
          }
          // First writer wins per lang — listTags is sorted by name, so
          // this is arbitrary; representative-aware lookup would require
          // an extra fetch but the per-lang map already narrows to one.
          if (!g.has(t.lang)) g.set(t.lang, t.id);
        }
      }
      this.#byId = byId;
      this.#byTag = byTag;
      this.#loaded = true;
    } finally {
      this.#loading = false;
    }
  }

  /** Force a re-fetch — call after creating/renaming a label. */
  async refresh(limit = 500): Promise<void> {
    this.#loaded = false;
    await this.ensure(limit);
  }

  /**
   * Return the display string for a label id OR a concept tag_id in the
   * current UI locale. Accepts both forms because `content_teaches` /
   * `content_prereqs` store concept ids (`tg-…`), while caller-hand-built
   * lookups may pass label ids (e.g., "Abstract Algebra"). Resolution:
   * sibling in UI locale → sibling in en → the label's own id (which IS
   * the display for its own lang).
   */
  localize(id: string): string {
    const locale = getLocale();
    let tag = this.#byId.get(id);
    if (!tag) {
      // Treat `id` as a concept tag_id: pick the best-matching sibling label.
      const siblings = this.#byTag.get(id);
      if (siblings) {
        const labelId = siblings.get(locale) ?? siblings.get('en') ?? siblings.values().next().value;
        if (labelId) tag = this.#byId.get(labelId);
      }
    }
    if (!tag) return id;
    if (tag.lang === locale) return tag.id;
    const siblings = this.#byTag.get(tag.tag_id);
    if (siblings) {
      const match = siblings.get(locale) ?? siblings.get('en');
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
