import { listTags, listMyNamePrefs } from './api';
import { getLocale } from './i18n/index.svelte';
import { getAuth } from './auth.svelte';
import type { Tag } from './types';

/**
 * In-memory name lookup used to render tag chips in the viewer's
 * locale. A concept (`tag_id`) has many `Tag` rows (names); the one
 * we show is chosen by:
 *
 *   1. the viewer's `user_name_pref` for that tag, if any; else
 *   2. the earliest-added name in the UI locale; else
 *   3. the earliest-added name in en; else
 *   4. any earliest-added name.
 *
 * Usage from a component:
 *   import { tagStore } from '../lib/tagStore.svelte';
 *   $effect(() => { tagStore.ensure(); });
 *   ... {tagStore.localize(tagId)}
 */
class TagStore {
  #byName   = $state(new Map<string, Tag>());                       // name_id → Tag
  #byTag    = $state(new Map<string, Map<string, string[]>>());     // tag_id → lang → [name_id, …] earliest-first
  #myPref   = $state(new Map<string, string>());                    // tag_id → name_id
  #loaded   = $state(false);
  #loading  = $state(false);

  /** Fetch names + viewer preferences. Idempotent. */
  async ensure(limit = 500): Promise<void> {
    if (this.#loaded || this.#loading) return;
    this.#loading = true;
    try {
      const tags = await listTags(limit);
      const byName = new Map<string, Tag>();
      const byTag = new Map<string, Map<string, string[]>>();
      // listTags orders by name — re-sort by added_at so earliest-first
      // per (tag_id, lang) is preserved in #byTag[tag][lang] arrays.
      const sorted = [...tags].sort((a, b) => a.added_at.localeCompare(b.added_at));
      for (const t of sorted) {
        byName.set(t.id, t);
        if (!t.tag_id) continue;
        let langs = byTag.get(t.tag_id);
        if (!langs) { langs = new Map(); byTag.set(t.tag_id, langs); }
        const arr = langs.get(t.lang) ?? [];
        arr.push(t.id);
        langs.set(t.lang, arr);
      }
      this.#byName = byName;
      this.#byTag = byTag;

      // Overlay user pref if authenticated; silently skip on failure.
      if (getAuth()) {
        try {
          const prefs = await listMyNamePrefs();
          this.#myPref = new Map(Object.entries(prefs));
        } catch { /* leave empty */ }
      }
      this.#loaded = true;
    } finally {
      this.#loading = false;
    }
  }

  /** Re-fetch — call after adding/removing a name or changing a pref. */
  async refresh(limit = 500): Promise<void> {
    this.#loaded = false;
    await this.ensure(limit);
  }

  /**
   * Display string for a name id (`tn-…`) OR a concept tag_id (`tg-…`).
   * Resolution order: viewer pref → earliest-added in UI locale →
   * earliest-added in en → any → the raw input.
   */
  localize(id: string): string {
    const locale = getLocale();
    const tag = this.#resolveTag(id, locale);
    return tag ? tag.name : id;
  }

  /** Full Tag record for display-decision helpers. */
  get(id: string): Tag | undefined {
    return this.#byName.get(id);
  }

  /** The concept id backing whatever input the caller has. */
  tagIdOf(id: string): string | undefined {
    const t = this.#byName.get(id);
    if (t) return t.tag_id;
    return this.#byTag.has(id) ? id : undefined;
  }

  /** This viewer's current preferred name_id for `tag_id`, if set. */
  myPref(tag_id: string): string | undefined {
    return this.#myPref.get(tag_id);
  }

  /** Locally-cached pref update (call after successful API set). */
  setMyPref(tag_id: string, name_id: string | null): void {
    if (name_id === null) this.#myPref.delete(tag_id);
    else this.#myPref.set(tag_id, name_id);
    this.#myPref = new Map(this.#myPref);   // bust the $state reactivity
  }

  #resolveTag(id: string, locale: string): Tag | undefined {
    // Determine the tag_id.
    const tag_id = this.tagIdOf(id);
    if (!tag_id) return this.#byName.get(id);   // name id that's gone; fall through

    // 1. Viewer preference for this concept.
    const prefName = this.#myPref.get(tag_id);
    if (prefName) {
      const t = this.#byName.get(prefName);
      if (t) return t;
    }
    const langs = this.#byTag.get(tag_id);
    if (!langs) return this.#byName.get(id);

    // 2. Earliest in UI locale.
    const localeList = langs.get(locale);
    if (localeList && localeList.length > 0) return this.#byName.get(localeList[0]);
    // 3. Earliest in en.
    const enList = langs.get('en');
    if (enList && enList.length > 0) return this.#byName.get(enList[0]);
    // 4. Earliest in any lang.
    for (const list of langs.values()) {
      if (list.length > 0) return this.#byName.get(list[0]);
    }
    return undefined;
  }
}

export const tagStore = new TagStore();
