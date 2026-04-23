<script lang="ts">
  import { getProfile, getArticlesByDid, getQuestionsByDid, getAnswersByDid, listSeries, getAllArticleTeaches, getAllSeriesArticles, listFollows, followUser, unfollowUser, markFollowSeen, updateProfileContacts, getFollowing, getFollowers, getSettings, setSettings, blockUser as apiBlockUser, unblockUser as apiUnblockUser, createReport, listPublicBookmarks, updateEducation, updateExperience, updatePublications, updateProjects, updateTeaching, getUserListings, uploadAvatar, uploadBanner, updateBio, updateDisplayName } from '../lib/api';
  import type { FollowEntry } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { isBlocked, addBlocked, removeBlocked } from '../lib/blocklist.svelte';
  import { tagName, deduplicateByTranslation, deduplicateSeriesByTranslation } from '../lib/display';
  import { t, getLocale } from '../lib/i18n/index.svelte';
  import { buildSeriesArticleMaps, buildArticleRowMap } from '../lib/series';
  import { contentHref } from '../lib/utils';
  import PostCard from '../lib/components/PostCard.svelte';
  import type { ProfileData, Article, Series, ContentTeachRow, Contacts, ContactKind, CustomLink, LinkedHandle, BookmarkWithTitle, EducationEntry, EducationTranslation, WorkExperienceEntry, WorkExperienceTranslation, PublicationEntry, ProjectEntry, TeachingEntry, Listing } from '../lib/types';
  import { CONTACT_KINDS } from '../lib/types';

  /** Contact kinds that store {url, username} rather than a bare string. */
  const LINKED_HANDLE_KINDS = ['bilibili'] as const;
  function isLinkedHandleKind(k: ContactKind): k is 'bilibili' {
    return (LINKED_HANDLE_KINDS as readonly string[]).includes(k);
  }

  /** Resolve a localized field (Record<string, string>) to the current locale with fallback. */
  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const locale = getLocale();
    return field[locale] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  /** Resolve an education field by locale, falling back to the default value. */
  function eduField(edu: EducationEntry, field: keyof EducationTranslation, loc: string): string | null | undefined {
    const tr = edu.translations?.[loc];
    if (tr) {
      const val = tr[field];
      if (val) return val;
    }
    return edu[field];
  }

  /** Resolve a work-experience field by locale, falling back to the default value. */
  function expField(exp: WorkExperienceEntry, field: keyof WorkExperienceTranslation, loc: string): string | null | undefined {
    const tr = exp.translations?.[loc];
    if (tr) {
      const val = tr[field];
      if (val) return val;
    }
    return exp[field];
  }

  // All series (including sub-series) for building tree
  let allUserSeries = $state<Series[]>([]);

  let { did } = $props<{ did: string }>();

  let locale = $derived(getLocale());

  let profile = $state<ProfileData | null>(null);
  let articles = $state<Article[]>([]);

  let articleTeaches = $state(new Map<string, ContentTeachRow[]>());
  let seriesArticleUris = $state(new Set<string>());
  let seriesArticleMap = $state(new Map<string, string[]>());
  let loading = $state(true);
  let isFollowing = $state(false);
  let followLoading = $state(false);
  let editingContacts = $state(false);
  let editContacts = $state<Contacts>({});
  let editingBio = $state(false);
  let editBio = $state('');
  let editingName = $state(false);
  let editName = $state('');
  // Academic profile state
  let editingEdu = $state(false);
  let editEdu = $state<EducationEntry[]>([]);
  let editingExp = $state(false);
  let editExp = $state<WorkExperienceEntry[]>([]);
  let userListings = $state<Listing[]>([]);
  let editingPubs = $state(false);
  let editPubs = $state<PublicationEntry[]>([]);
  let editingProjects = $state(false);
  let editProj = $state<ProjectEntry[]>([]);
  let editingTeach = $state(false);
  let editTeach = $state<TeachingEntry[]>([]);
  let userBlocked = $state(false);
  let reportOpen = $state(false);
  let reportReason = $state('');

  let questions = $state<Article[]>([]);
  let answers = $state<Article[]>([]);
  let publicBookmarks = $state<BookmarkWithTitle[]>([]);
  let profileTab = $state<string>('general');

  let isOwnProfile = $derived(getAuth()?.did === did);
  let following = $state<FollowEntry[]>([]);
  let followers = $state<FollowEntry[]>([]);
  let showFollowTab = $state<'following' | 'followers' | null>(null);

  interface ProfileFeedItem {
    type: 'article' | 'series';
    article?: Article;
    series?: Series;
    articleCount?: number;
    sortDate: string;
  }


  // Track which series are expanded
  let expandedSeries = $state(new Set<string>());
  function toggleExpand(e: MouseEvent, seriesId: string) {
    e.preventDefault();
    e.stopPropagation();
    const next = new Set(expandedSeries);
    if (next.has(seriesId)) next.delete(seriesId);
    else next.add(seriesId);
    expandedSeries = next;
  }

  function buildFeed(categoryFilter: string): ProfileFeedItem[] {
    const items: ProfileFeedItem[] = [];
    const deduped = deduplicateByTranslation(articles, locale);
    for (const a of deduped) {
      if (!seriesArticleUris.has(a.at_uri) && (a.category || 'general') === categoryFilter) {
        items.push({ type: 'article', article: a, sortDate: a.created_at });
      }
    }
    for (const s of allUserSeries) {
      if ((s.category || 'general') !== categoryFilter) continue;
      const totalArticles = (seriesArticleMap.get(s.id) || []).length;
      items.push({ type: 'series', series: s, articleCount: totalArticles, sortDate: s.created_at });
    }
    items.sort((a, b) => b.sortDate.localeCompare(a.sortDate));
    return items;
  }

  // Dynamically derive all categories from user's articles and series
  let userCategories = $derived.by((): { key: string; label: string; count: number }[] => {
    const counts = new Map<string, number>();
    const deduped = deduplicateByTranslation(articles, locale);
    for (const a of deduped) {
      const cat = a.category || 'general';
      if (!seriesArticleUris.has(a.at_uri)) {
        counts.set(cat, (counts.get(cat) || 0) + 1);
      }
    }
    for (const s of allUserSeries) {
      const cat = s.category || 'general';
      counts.set(cat, (counts.get(cat) || 0) + 1);
    }
    // Build ordered list: 'general' first, then sorted by count desc
    const cats: { key: string; label: string; count: number }[] = [];
    const knownLabels: Record<string, string> = {
      general: t('category.general'),
      lecture: t('category.lecture'),
      paper: t('category.paper'),
      review: t('category.review'),
    };
    const allKeys = Array.from(counts.keys());
    // Ensure 'general' is always first
    if (!allKeys.includes('general')) allKeys.unshift('general');
    const sorted = allKeys.sort((a, b) => {
      if (a === 'general') return -1;
      if (b === 'general') return 1;
      return (counts.get(b) || 0) - (counts.get(a) || 0);
    });
    for (const key of sorted) {
      cats.push({
        key,
        label: knownLabels[key] || key,
        count: counts.get(key) || 0,
      });
    }
    return cats;
  });

  let currentFeed = $derived(buildFeed(profileTab));

  // "全部文章" tab: articles grouped by series
  interface ArticleGroup {
    series: Series | null;
    articles: Article[];
  }
  let allArticleGroups = $derived.by((): ArticleGroup[] => {
    const deduped = deduplicateByTranslation(articles, locale);
    const groups: ArticleGroup[] = [];
    const assignedUris = new Set<string>();

    for (const s of allUserSeries) {
      const uriList = seriesArticleMap.get(s.id) || [];
      const seriesArts = uriList
        .map(uri => deduped.find(a => a.at_uri === uri))
        .filter(Boolean) as Article[];
      if (seriesArts.length > 0) {
        groups.push({ series: s, articles: seriesArts });
        seriesArts.forEach(a => assignedUris.add(a.at_uri));
      }
    }

    const standalone = deduped.filter(a => !assignedUris.has(a.at_uri));
    if (standalone.length > 0) {
      groups.push({ series: null, articles: standalone });
    }

    return groups;
  });

  $effect(() => {
    load();
  });

  async function load() {
    loading = true;
    try {
      const [prof, arts, qs, ans, allSeries, tags, seriesArts] = await Promise.all([
        getProfile(did),
        getArticlesByDid(did),
        getQuestionsByDid(did),
        getAnswersByDid(did),
        listSeries(),
        getAllArticleTeaches(),
        getAllSeriesArticles(),
      ]);
      profile = prof;
      document.title = `${prof.display_name || '@' + prof.handle} — NightBoat`;
      getUserListings(did).then(l => { userListings = l; }).catch(() => {});
      articles = arts;
      questions = qs;
      answers = ans;
      allUserSeries = deduplicateSeriesByTranslation(allSeries.filter(s => s.created_by === did), getLocale());

      const saMaps = buildSeriesArticleMaps(seriesArts);
      seriesArticleUris = saMaps.seriesArticleUris;
      seriesArticleMap = saMaps.seriesArticleMap;

      articleTeaches = buildArticleRowMap(tags);

      // Load following/followers
      const [fg, fr] = await Promise.all([getFollowing(did), getFollowers(did)]);
      following = fg;
      followers = fr;

      // Check block + follow status
      userBlocked = isBlocked(did);
      if (getAuth() && !isOwnProfile) {
        try {
          const follows = await listFollows();
          isFollowing = follows.some(f => f.follows_did === did);
          // Mark as seen
          if (isFollowing) markFollowSeen(did).catch(() => {});
        } catch { /* */ }
      }

      // Load public bookmarks (for other users)
      if (!isOwnProfile) {
        try {
          publicBookmarks = await listPublicBookmarks(did);
        } catch { publicBookmarks = []; }
      }
    } catch { /* */ }
    loading = false;
  }

  async function toggleFollow() {
    followLoading = true;
    try {
      if (isFollowing) {
        await unfollowUser(did);
        isFollowing = false;
      } else {
        await followUser(did);
        isFollowing = true;
      }
    } catch { /* */ }
    followLoading = false;
  }

  function startEditContacts() {
    const src: Contacts = profile?.contacts || {};
    editContacts = {
      ...src,
      // Ensure LinkedHandle kinds always have a writable object so <input>
      // bindings don't crash when the user has nothing saved yet.
      bilibili: src.bilibili ? { ...src.bilibili } : { url: '', username: '' },
      custom: [...(src.custom || []).map(l => ({ ...l }))],
    };
    editingContacts = true;
  }

  function contactPlaceholder(kind: ContactKind): string {
    const hints: Record<ContactKind, string> = {
      website: 'https://example.com',
      email: 'you@example.com',
      telegram: '@username',
      matrix: '@user:matrix.org',
      bluesky: 'handle.bsky.social',
      github: 'username',
      codeberg: 'username',
      // bilibili uses paired username+url inputs, not this placeholder.
      youtube: '@channel',
      bilibili: '',
    };
    return hints[kind];
  }

  async function saveContacts() {
    const cleaned: Contacts = {};
    for (const k of CONTACT_KINDS) {
      if (isLinkedHandleKind(k)) {
        const lh = editContacts[k] as LinkedHandle | undefined;
        const url = lh?.url.trim() || '';
        const username = lh?.username.trim() || '';
        if (url || username) cleaned[k] = { url, username };
      } else {
        const v = (editContacts[k] as string | undefined)?.trim() || '';
        if (v) (cleaned as Record<string, unknown>)[k] = v;
      }
    }
    const custom = (editContacts.custom || [])
      .map(l => ({ label: l.label.trim(), url: l.url.trim() }))
      .filter(l => l.label && l.url);
    if (custom.length > 0) cleaned.custom = custom;
    try {
      await updateProfileContacts(cleaned);
      if (profile) profile.contacts = cleaned;
      editingContacts = false;
    } catch { /* */ }
  }

  /** Build the `href` for a contact value, adding a protocol/prefix as needed. */
  function contactHref(kind: ContactKind, value: string): string {
    switch (kind) {
      case 'email': return value.startsWith('mailto:') ? value : `mailto:${value}`;
      case 'telegram': {
        if (value.startsWith('http')) return value;
        const u = value.replace(/^@/, '');
        return `https://t.me/${u}`;
      }
      case 'matrix': {
        if (value.startsWith('http')) return value;
        const id = value.startsWith('@') ? value : `@${value}`;
        return `https://matrix.to/#/${encodeURIComponent(id)}`;
      }
      case 'github':
        return value.startsWith('http') ? value : `https://github.com/${value.replace(/^@/, '')}`;
      case 'codeberg':
        return value.startsWith('http') ? value : `https://codeberg.org/${value.replace(/^@/, '')}`;
      case 'bluesky':
        return value.startsWith('http') ? value : `https://bsky.app/profile/${value.replace(/^@/, '')}`;
      case 'youtube': {
        if (value.startsWith('http')) return value;
        // Accept @handle, channel/..., or plain user id
        const v = value.replace(/^@/, '');
        return `https://youtube.com/@${v}`;
      }
      case 'bilibili':
        return value.startsWith('http') ? value : `https://${value.replace(/^\/+/, '')}`;
      case 'website':
      default:
        return value.startsWith('http') ? value : `https://${value}`;
    }
  }

  function contactLabel(kind: ContactKind): string {
    const map: Record<ContactKind, string> = {
      website: 'Website', email: 'Email', telegram: 'Telegram',
      matrix: 'Matrix', bluesky: 'Bluesky',
      github: 'GitHub', codeberg: 'Codeberg',
      youtube: 'YouTube', bilibili: 'Bilibili',
    };
    return map[kind];
  }

  let hasAnyContact = $derived(
    CONTACT_KINDS.some(k => {
      const v = profile?.contacts?.[k];
      if (!v) return false;
      if (typeof v === 'string') return v.length > 0;
      return v.url.length > 0 || v.username.length > 0;
    })
      || (profile?.contacts?.custom?.length ?? 0) > 0
  );

  function addCustomLink() {
    editContacts = {
      ...editContacts,
      custom: [...(editContacts.custom || []), { label: '', url: '' }],
    };
  }
  function removeCustomLink(idx: number) {
    editContacts = {
      ...editContacts,
      custom: (editContacts.custom || []).filter((_, i) => i !== idx),
    };
  }

  async function toggleBlock() {
    if (userBlocked) {
      try {
        await apiUnblockUser(did);
        removeBlocked(did);
        userBlocked = false;
      } catch { /* */ }
    } else {
      if (!confirm(t('block.confirm'))) return;
      try {
        await apiBlockUser(did);
        addBlocked(did);
        userBlocked = true;
      } catch { /* */ }
    }
  }

  async function submitReport() {
    if (!reportReason.trim()) return;
    try {
      await createReport(did, 'user', reportReason.trim());
      reportOpen = false;
      reportReason = '';
      alert(t('report.success'));
    } catch {
      alert(t('report.failed'));
    }
  }

  function shortDid(d: string) {
    return d.replace('did:plc:', '').slice(0, 12);
  }
</script>

{#snippet contactIcon(kind: ContactKind | 'custom')}
  {#if kind === 'website'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
  {:else if kind === 'email'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/><polyline points="22,6 12,13 2,6"/></svg>
  {:else if kind === 'telegram'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/></svg>
  {:else if kind === 'matrix'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M.632.55v22.9H2.28V24H0V0h2.28v.55zm7.043 7.26v1.157h.033a3.312 3.312 0 0 1 1.117-1.024c.433-.245.936-.367 1.5-.367.54 0 1.033.107 1.481.314.448.208.785.582 1.02 1.108.254-.374.6-.706 1.034-.992.434-.287.95-.43 1.546-.43.453 0 .872.056 1.26.167.388.11.716.286.993.525.276.24.489.548.646.93.157.38.236.85.236 1.406v5.686h-2.322v-4.81c0-.29-.012-.562-.034-.82a1.798 1.798 0 0 0-.176-.685 1.089 1.089 0 0 0-.412-.459c-.184-.116-.436-.174-.755-.174-.313 0-.566.07-.76.21a1.338 1.338 0 0 0-.445.542c-.108.217-.18.464-.215.74-.034.276-.05.53-.05.762v4.692h-2.324v-5.025c0-.298-.012-.566-.034-.806-.022-.24-.084-.441-.186-.608a.984.984 0 0 0-.403-.409c-.174-.098-.424-.148-.748-.148-.205 0-.398.042-.58.125-.18.083-.356.213-.52.393-.165.18-.297.392-.395.634-.099.243-.148.517-.148.827v5.017H5.36V7.81zm15.693 15.64V.55H21.72V0H24v24h-2.28v-.55z"/></svg>
  {:else if kind === 'bluesky'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M12 10.8c-1.087-2.114-4.046-6.053-6.798-7.995C2.566.944 1.561 1.266.902 1.565.139 1.908 0 3.08 0 3.768c0 .69.378 5.65.624 6.479.815 2.736 3.713 3.66 6.383 3.364.136-.02.275-.039.415-.056-.138.022-.276.04-.415.056-3.911.58-7.386 2.005-2.83 7.078 5.013 5.19 6.87-1.113 7.823-4.308.953 3.195 2.05 9.271 7.733 4.308 4.267-4.308 1.172-6.498-2.74-7.078a8.741 8.741 0 0 1-.415-.056c.14.017.279.036.415.056 2.67.297 5.568-.628 6.383-3.364.246-.828.624-5.79.624-6.478 0-.69-.139-1.861-.902-2.206-.659-.298-1.664-.62-4.3 1.24C16.046 4.748 13.087 8.687 12 10.8Z"/></svg>
  {:else if kind === 'github'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.52 11.52 0 0 1 12 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
  {:else if kind === 'codeberg'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M11.955.49A12 12 0 0 0 0 12.49a12 12 0 0 0 6.568 10.692L11.97 13.46v-.002l-.982-6.56a.313.313 0 0 1 .612-.12l.4 1.26-.005.007 1.86 5.874-.003.001 4.162 9.443a12 12 0 0 0 6.022-10.416A12 12 0 0 0 11.955.49z"/></svg>
  {:else if kind === 'youtube'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z"/></svg>
  {:else if kind === 'bilibili'}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.658.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"/></svg>
  {:else}
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>
  {/if}
{/snippet}

{#snippet contactsBlock()}
  {#if hasAnyContact || isOwnProfile}
    <div class="contacts-list">
      {#each CONTACT_KINDS as kind}
        {@const v = profile?.contacts?.[kind]}
        {#if v}
          {#if typeof v === 'string'}
            <a href={contactHref(kind, v)} target="_blank" rel="noopener" class="contact-row" title={contactLabel(kind)}>
              <span class="contact-icon">{@render contactIcon(kind)}</span>
              <span class="contact-value">{v}</span>
            </a>
          {:else if v.url || v.username}
            <a href={v.url ? contactHref(kind, v.url) : '#'} target="_blank" rel="noopener" class="contact-row" title={contactLabel(kind)}>
              <span class="contact-icon">{@render contactIcon(kind)}</span>
              <span class="contact-value">{v.username || v.url}</span>
            </a>
          {/if}
        {/if}
      {/each}
      {#each (profile?.contacts?.custom || []) as link}
        <a href={link.url} target="_blank" rel="noopener" class="contact-row" title={link.label}>
          <span class="contact-icon">{@render contactIcon('custom')}</span>
          <span class="contact-value">{link.label}</span>
        </a>
      {/each}
      {#if isOwnProfile}
        <button class="contacts-edit-btn" onclick={startEditContacts}>
          {hasAnyContact ? t('common.edit') : t('profile.addContacts')}
        </button>
      {/if}
    </div>
  {/if}
{/snippet}

{#snippet seriesTree(s: Series, totalArticles?: number)}
  {@const articleUris = seriesArticleMap.get(s.id) || []}
  {@const count = totalArticles ?? articleUris.length}
  {@const isExpanded = expandedSeries.has(s.id)}

  <div class="series-tree-node">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="series-tree-card" onclick={(e) => count > 0 ? toggleExpand(e, s.id) : null}>
      <div class="series-tree-top">
        {#if count > 0}
          <span class="expand-arrow" class:expanded={isExpanded}>&#9654;</span>
        {:else}
          <span class="expand-arrow-placeholder"></span>
        {/if}
        <a href="/series?id={encodeURIComponent(s.id)}" class="series-tree-title" onclick={(e) => e.stopPropagation()}>
          {s.title}
        </a>
        <span class="series-badge">{t('profile.seriesBadge')}</span>
      </div>
      {#if s.summary}
        <p class="series-tree-desc">{s.summary}</p>
      {/if}
      <div class="series-tree-bottom">
        <span class="post-meta">{s.created_at.split(' ')[0]}</span>
        <span class="card-stats">
          <span class="stat">{count} {t('profile.lectureCount')}</span>
        </span>
      </div>
    </div>

    {#if isExpanded}
      {#each articleUris as uri}
        {@const art = articles.find(a => a.at_uri === uri)}
        {#if art}
          <PostCard
            article={art}
            articleTeaches={articleTeaches.get(art.at_uri) || []}
           
          />
        {/if}
      {/each}
    {/if}
  </div>
{/snippet}

{#if loading}
  <p class="meta">Loading...</p>
{:else if profile}
  <div class="banner-wrap">
    {#if profile.banner_url}
      <img src={profile.banner_url} alt="" class="banner-img" />
    {:else}
      <div class="banner-placeholder"></div>
    {/if}
    {#if isOwnProfile}
      <label class="banner-upload" title={t('profile.uploadBanner')}>
        <input type="file" accept="image/*" class="sr-only" onchange={async (e) => {
          const file = (e.target as HTMLInputElement).files?.[0];
          if (!file) return;
          try {
            const url = await uploadBanner(file);
            profile!.banner_url = url;
          } catch { /* */ }
        }} />
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/></svg>
      </label>
    {/if}
  </div>
  <div class="profile-header">
    <div class="avatar-wrap">
      {#if profile.avatar_url}
        <img src={profile.avatar_url} alt="avatar" class="avatar" />
      {:else}
        <div class="avatar placeholder">{(profile.handle || profile.did).charAt(0).toUpperCase()}</div>
      {/if}
      {#if isOwnProfile}
        <label class="avatar-upload" title={t('profile.uploadAvatar')}>
          <input type="file" accept="image/*" class="sr-only" onchange={async (e) => {
            const file = (e.target as HTMLInputElement).files?.[0];
            if (!file) return;
            try {
              const url = await uploadAvatar(file);
              profile!.avatar_url = url;
            } catch { /* */ }
          }} />
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/></svg>
        </label>
      {/if}
    </div>
    <div class="profile-info">
      {#if editingName}
        <div class="name-edit">
          <input class="name-input" bind:value={editName} placeholder={t('profile.displayNamePlaceholder')} />
          <button class="email-save" onclick={async () => { await updateDisplayName(editName.trim()); if (profile) profile.display_name = editName.trim() || null; editingName = false; }}>{t('common.save')}</button>
          <button class="email-cancel" onclick={() => { editingName = false; }}>{t('common.cancel')}</button>
        </div>
      {:else}
        <h1 class="display-name">
          {profile.display_name || profile.handle || shortDid(profile.did)}
          {#if isOwnProfile}
            <button class="edit-name-btn" onclick={() => { editName = profile?.display_name || ''; editingName = true; }}>✎</button>
          {/if}
        </h1>
      {/if}
      {#if profile.handle}
        <p class="handle">
          @{profile.handle}
          {#if (profile.did.startsWith('did:plc:') || profile.did.startsWith('did:web:')) && profile.handle}
            <a
              href="https://bsky.app/profile/{profile.handle}"
              target="_blank"
              rel="noopener"
              class="handle-bsky"
              title="Bluesky"
              aria-label="Bluesky profile"
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M12 10.8c-1.087-2.114-4.046-6.053-6.798-7.995C2.566.944 1.561 1.266.902 1.565.139 1.908 0 3.08 0 3.768c0 .69.378 5.65.624 6.479.815 2.736 3.713 3.66 6.383 3.364-3.911.58-7.386 2.005-2.83 7.078 5.013 5.19 6.87-1.113 7.823-4.308.953 3.195 2.05 9.271 7.733 4.308 4.267-4.308 1.172-6.498-2.74-7.078 2.67.297 5.568-.628 6.383-3.364.246-.828.624-5.79.624-6.478 0-.69-.139-1.861-.902-2.206-.659-.298-1.664-.62-4.3 1.24C16.046 4.748 13.087 8.687 12 10.8Z"/></svg>
            </a>
          {/if}
        </p>
      {/if}
      {#if editingBio}
        <div class="bio-edit">
          <textarea class="bio-input" bind:value={editBio} placeholder={t('settings.bioPlaceholder')} rows="3"></textarea>
          <div class="bio-edit-actions">
            <button class="email-save" onclick={async () => { await updateBio(editBio.trim()); if (profile) profile.bio = editBio.trim(); editingBio = false; }}>{t('common.save')}</button>
            <button class="email-cancel" onclick={() => { editingBio = false; }}>{t('common.cancel')}</button>
          </div>
        </div>
      {:else if profile.bio || isOwnProfile}
        <div class="bio">
          {#if profile.bio}{profile.bio}{:else}<span class="bio-placeholder">{t('settings.bioPlaceholder')}</span>{/if}
          {#if isOwnProfile}
            <button class="edit-email-btn" onclick={() => { editBio = profile?.bio || ''; editingBio = true; }}>{t('common.edit')}</button>
          {/if}
        </div>
      {/if}
      {#if profile.affiliation}
        <p class="credential-line">
          {profile.affiliation}
          {#if profile.credentials_verified}
            <span class="verified-badge" title={t('profile.verified')}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--accent)" stroke="white" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
            </span>
          {/if}
        </p>
      {/if}
      <div class="profile-columns">
        <div class="profile-col contacts-col">
          {@render contactsBlock()}
        </div>
        <div class="profile-col education-col">
          {#if profile.education.length > 0 || isOwnProfile}
            <h4 class="col-heading">{t('profile.education')}</h4>
            <div class="education-list">
              {#each [...profile.education].sort((a, b) => (b.start_date || '').localeCompare(a.start_date || '')) as edu}
                {@const school = eduField(edu, 'school', locale) || edu.school}
                {@const dept = eduField(edu, 'department', locale)}
                {@const major = eduField(edu, 'major', locale)}
                <div class="education-entry">
                  <span class="edu-degree">{t('profile.degree.' + edu.degree) || edu.degree}</span>
                  <span class="edu-school">{school}{#if dept}, {dept}{/if}</span>
                  {#if major}<span class="edu-major">{major}</span>{/if}
                  <span class="edu-dates">
                    {edu.start_date || ''}{#if edu.start_date} – {/if}{#if edu.current}{t('profile.present') || 'Present'}{:else}{edu.end_date || ''}{/if}
                  </span>
                </div>
              {/each}
              {#if profile.credentials_verified && !profile.affiliation}
                <span class="verified-badge" title={t('profile.verified')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="var(--accent)" stroke="white" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
                </span>
              {/if}
              {#if isOwnProfile}
                <button class="edit-section-btn" onclick={() => { editEdu = JSON.parse(JSON.stringify(profile!.education)); editingEdu = true; }}>
                  {profile.education.length > 0 ? t('common.edit') : t('profile.add')}
                </button>
              {/if}
            </div>
          {/if}
        </div>
        <div class="profile-col experience-col">
          {#if profile.experience.length > 0 || isOwnProfile}
            <h4 class="col-heading">{t('profile.experience')}</h4>
            <div class="experience-list">
              {#each [...profile.experience].sort((a, b) => (b.start_date || '').localeCompare(a.start_date || '')) as exp}
                {@const company = expField(exp, 'company', locale) || exp.company}
                {@const dept = expField(exp, 'department', locale)}
                {@const title = expField(exp, 'title', locale)}
                {@const location = expField(exp, 'location', locale)}
                {@const desc = expField(exp, 'description', locale)}
                <div class="experience-entry">
                  <span class="exp-company">{company}{#if dept}, {dept}{/if}</span>
                  {#if title}<span class="exp-title">{title}</span>{/if}
                  {#if location}<span class="exp-location">{location}</span>{/if}
                  <span class="edu-dates">
                    {exp.start_date || ''}{#if exp.start_date} – {/if}{#if exp.current}{t('profile.present')}{:else}{exp.end_date || ''}{/if}
                  </span>
                  {#if desc}<p class="exp-desc">{desc}</p>{/if}
                </div>
              {/each}
              {#if isOwnProfile}
                <button class="edit-section-btn" onclick={() => { editExp = JSON.parse(JSON.stringify(profile!.experience)); editingExp = true; }}>
                  {profile.experience.length > 0 ? t('common.edit') : t('profile.add')}
                </button>
              {/if}
            </div>
          {/if}
        </div>
      </div>
      <div class="profile-stats">
        <span class="rep-stat" title={t('profile.reputationFull')}><strong>{profile.reputation.toLocaleString()}</strong> {t('profile.reputation')}</span>
        <span>{profile.article_count} {t('profile.articles')}</span>
        <span>{profile.series_count} {t('profile.seriesCount')}</span>
        <button class="stat-btn" onclick={() => { showFollowTab = showFollowTab === 'following' ? null : 'following'; }}>
          <strong>{following.length}</strong> {t('profile.following')}
        </button>
        <button class="stat-btn" onclick={() => { showFollowTab = showFollowTab === 'followers' ? null : 'followers'; }}>
          <strong>{followers.length}</strong> {t('profile.followers')}
        </button>
      </div>
    </div>
    <div class="profile-actions-row">
      {#if getAuth() && !isOwnProfile}
        <button
          class="follow-btn"
          class:following={isFollowing}
          onclick={toggleFollow}
          disabled={followLoading}
        >
          {isFollowing ? t('profile.unfollow') : t('profile.follow')}
        </button>
      {/if}
      {#if isOwnProfile}
        <a href="/settings" class="settings-link">{t('profile.settings')}</a>
      {/if}
      <a href="/feed/{encodeURIComponent(did)}.xml" class="rss-link" title={t('profile.rssTitle')} target="_blank" rel="noopener">RSS</a>
    </div>
    {#if getAuth() && !isOwnProfile}
      <div class="profile-actions-secondary">
        <button class="action-btn" class:active={userBlocked} onclick={toggleBlock}>
          {userBlocked ? t('block.unblock') : t('block.block')}
        </button>
        <button class="action-btn" onclick={() => { reportOpen = true; }}>
          {t('report.report')}
        </button>
      </div>
    {/if}
  </div>

  <!-- Report modal -->
  {#if reportOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="links-overlay" onclick={() => { reportOpen = false; }}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="links-modal" onclick={(e) => e.stopPropagation()}>
        <h3>{t('report.title')}</h3>
        <p class="report-target">
          {t('report.kindUser')}: {profile?.display_name || profile?.handle || did}
        </p>
        <textarea
          bind:value={reportReason}
          placeholder={t('report.reasonPlaceholder')}
          class="report-textarea"
          rows="4"
        ></textarea>
        <div class="link-actions">
          <button class="link-cancel" onclick={() => { reportOpen = false; }}>{t('common.cancel')}</button>
          <button class="link-save" onclick={submitReport} disabled={!reportReason.trim()}>{t('report.submit')}</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Edit contacts modal -->
  {#if editingContacts}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="links-overlay" onclick={() => { editingContacts = false; }}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="links-modal" onclick={(e) => e.stopPropagation()}>
        <h3>{t('profile.editContactsTitle')}</h3>
        <div class="contacts-edit-grid">
          {#each CONTACT_KINDS as kind}
            <div class="contact-edit-row">
              <span class="contact-edit-icon">{@render contactIcon(kind)}</span>
              <span class="contact-edit-label">{contactLabel(kind)}</span>
              {#if kind === 'bilibili'}
                <div class="contact-edit-paired">
                  <input class="contact-edit-input" bind:value={editContacts.bilibili!.username} placeholder={t('profile.contactUsername')} />
                  <input class="contact-edit-input" bind:value={editContacts.bilibili!.url} placeholder="https://space.bilibili.com/..." />
                </div>
              {:else}
                <input
                  class="contact-edit-input"
                  bind:value={editContacts[kind] as string}
                  placeholder={contactPlaceholder(kind)}
                />
              {/if}
            </div>
          {/each}
        </div>
        <h4 class="contacts-custom-heading">{t('profile.contactsCustom')}</h4>
        {#each (editContacts.custom || []) as _link, i}
          <div class="link-add-row">
            <input bind:value={editContacts.custom![i].label} placeholder={t('profile.linkLabel')} />
            <input bind:value={editContacts.custom![i].url} placeholder="https://..." />
            <button class="link-remove" onclick={() => removeCustomLink(i)}>&times;</button>
          </div>
        {/each}
        <button class="add-entry" onclick={addCustomLink}>+ {t('profile.contactsAddCustom')}</button>
        <div class="link-actions">
          <button class="link-cancel" onclick={() => { editingContacts = false; }}>{t('common.cancel')}</button>
          <button class="link-save" onclick={saveContacts}>{t('common.save')}</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showFollowTab}
    {@const list = showFollowTab === 'following' ? following : followers}
    <div class="follow-list">
      <h3 class="section-title">{showFollowTab === 'following' ? t('profile.following') : t('profile.followers')}</h3>
      {#if list.length === 0}
        <p class="empty-text">{t('profile.none')}</p>
      {:else}
        {#each list as u}
          <a href="/profile?did={encodeURIComponent(u.did)}" class="follow-item">
            {#if u.avatar_url}
              <img src={u.avatar_url} alt="" class="follow-avatar" />
            {:else}
              <div class="follow-avatar placeholder">{(u.handle || u.did).charAt(0).toUpperCase()}</div>
            {/if}
            <div class="follow-info">
              <span class="follow-name">{u.display_name || u.handle || u.did.slice(0, 20)}</span>
              {#if u.handle}
                <span class="follow-handle">@{u.handle}</span>
              {/if}
            </div>
          </a>
        {/each}
      {/if}
    </div>
  {/if}

  <div class="profile-body">
    <main class="profile-main">
      <div class="profile-tabs">
        {#each userCategories as cat (cat.key)}
          {#if cat.count > 0 || cat.key === 'general' || isOwnProfile}
            <button class="tab-btn" class:active={profileTab === cat.key} onclick={() => { profileTab = cat.key; }}>
              {cat.label}
              {#if cat.count > 0}<span class="tab-count">{cat.count}</span>{/if}
            </button>
          {/if}
        {/each}
        <button class="tab-btn" class:active={profileTab === 'qa'} onclick={() => { profileTab = 'qa'; }}>
          {t('profile.questions')}
          {#if questions.length + answers.length > 0}
            <span class="tab-count">{questions.length + answers.length}</span>
          {/if}
        </button>
        {#if !isOwnProfile && publicBookmarks.length > 0}
          <button class="tab-btn" class:active={profileTab === 'bookmarks'} onclick={() => { profileTab = 'bookmarks'; }}>
            {t('profile.publicBookmarks')}
            <span class="tab-count">{publicBookmarks.length}</span>
          </button>
        {/if}
        {#if articles.length > 0}
          <button class="tab-btn" class:active={profileTab === 'all'} onclick={() => { profileTab = 'all'; }}>
            {t('profile.tabAllArticles')}
            <span class="tab-count">{articles.length}</span>
          </button>
        {/if}
      </div>

      {#if profileTab !== 'qa' && profileTab !== 'bookmarks' && profileTab !== 'all'}
        {#each currentFeed as item}
          {#if item.type === 'article' && item.article}
            <PostCard
              article={item.article}
              articleTeaches={articleTeaches.get(item.article.at_uri) || []}
            />
          {:else if item.type === 'series' && item.series}
            {@render seriesTree(item.series, item.articleCount)}
          {/if}
        {/each}

        {#if currentFeed.length === 0}
          <p class="empty-text">{t('profile.noWorks')}</p>
        {/if}

        {#if isOwnProfile}
          <div class="create-actions">
            <a href="/new" class="create-link">{t('profile.writeArticle')}</a>
            <a href="/new-series" class="create-link">{t('profile.createSeries')}</a>
          </div>
        {/if}
      {:else if profileTab === 'qa'}
        {#if questions.length > 0}
          <h3 class="section-title">{t('qa.myQuestions')}</h3>
          {#each questions as q}
            <a href="/question?uri={encodeURIComponent(q.at_uri)}" class="qa-card question">
              <span class="qa-badge question-badge">{t('qa.questionBadge')}</span>
              <span class="qa-title">{q.title}</span>
              <span class="qa-stat">{t('qa.answerCount', q.answer_count)}</span>
            </a>
          {/each}
        {/if}

        {#if answers.length > 0}
          <h3 class="section-title">{t('qa.myAnswers')}</h3>
          {#each answers as a}
            <a href="/question?uri={encodeURIComponent(a.question_uri || '')}" class="qa-card answer">
              <span class="qa-badge answer-badge">{t('qa.answerBadge')}</span>
              <span class="qa-title">{a.title}</span>
              <span class="qa-stat">&#9650; {a.vote_score}</span>
            </a>
          {/each}
        {/if}

        {#if questions.length === 0 && answers.length === 0}
          <p class="empty-text">{t('qa.noQuestions')}</p>
        {/if}

        {#if isOwnProfile}
          <div class="create-actions">
            <a href="/new-question" class="create-link">{t('qa.askQuestion')}</a>
          </div>
        {/if}
      {:else if profileTab === 'bookmarks'}
        {#each publicBookmarks as bm}
          <a href={contentHref(bm.article_uri, bm.kind, bm.question_uri)} class="bookmark-card">
            <div class="bookmark-info">
              <span class="bookmark-title">{bm.title}</span>
              {#if bm.folder_path && bm.folder_path !== '/'}
                <span class="bookmark-folder">{bm.folder_path}</span>
              {/if}
            </div>
            <span class="bookmark-date">{bm.created_at.split(' ')[0]}</span>
          </a>
        {/each}
        {#if publicBookmarks.length === 0}
          <p class="empty-text">{t('profile.noWorks')}</p>
        {/if}
      {:else if profileTab === 'all'}
        {#each allArticleGroups as group}
          {#if group.series}
            <div class="all-series-group">
              <a href="/series?id={group.series.id}" class="all-series-title">
                {group.series.title}
                <span class="all-series-count">{group.articles.length} 篇</span>
              </a>
              <div class="all-series-articles">
                {#each group.articles as art}
                  <a href="/article?uri={encodeURIComponent(art.at_uri)}" class="all-article-row">
                    <span class="all-article-title">{art.title || '（无标题）'}</span>
                    {#if art.lang}<span class="all-article-lang">{art.lang}</span>{/if}
                  </a>
                {/each}
              </div>
            </div>
          {:else}
            {#each group.articles as art}
              <PostCard article={art} articleTeaches={articleTeaches.get(art.at_uri) || []} />
            {/each}
          {/if}
        {/each}
        {#if allArticleGroups.length === 0}
          <p class="empty-text">{t('profile.noWorks')}</p>
        {/if}
      {/if}
    </main>

    <aside class="profile-sidebar">
      <!-- Publications -->
      {#if profile.publications.length > 0 || isOwnProfile}
        <div class="sidebar-card">
          <div class="section-header">
            <h3>{t('profile.publications')}</h3>
            {#if isOwnProfile}
              <button class="edit-section-btn" onclick={() => { editPubs = JSON.parse(JSON.stringify(profile!.publications)); editingPubs = true; }}>
                {profile.publications.length > 0 ? t('common.edit') : t('profile.add')}
              </button>
            {/if}
          </div>
          {#each profile.publications.sort((a, b) => b.year - a.year) as pub_entry}
            <div class="pub-entry">
              <span class="pub-authors">{pub_entry.authors.join(', ')}</span>
              {#if pub_entry.url || pub_entry.doi}
                <a href={pub_entry.url || `https://doi.org/${pub_entry.doi}`} target="_blank" rel="noopener" class="pub-title">"{loc(pub_entry.title)}"</a>
              {:else}
                <span class="pub-title">"{loc(pub_entry.title)}"</span>
              {/if}
              {#if pub_entry.venue}<span class="pub-venue">{loc(pub_entry.venue)}</span>{/if}
              {#if pub_entry.year}<span class="pub-year">({pub_entry.year})</span>{/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- Projects -->
      {#if profile.projects.length > 0 || isOwnProfile}
        <div class="sidebar-card">
          <div class="section-header">
            <h3>{t('profile.projects')}</h3>
            {#if isOwnProfile}
              <button class="edit-section-btn" onclick={() => { editProj = JSON.parse(JSON.stringify(profile!.projects)); editingProjects = true; }}>
                {profile.projects.length > 0 ? t('common.edit') : t('profile.add')}
              </button>
            {/if}
          </div>
          {#each profile.projects as proj}
            <div class="proj-entry">
              <div class="proj-top">
                {#if proj.url}
                  <a href={proj.url} target="_blank" rel="noopener" class="proj-title">{loc(proj.title)}</a>
                {:else}
                  <span class="proj-title">{loc(proj.title)}</span>
                {/if}
                <span class="status-badge status-{proj.status}">{proj.status}</span>
              </div>
              {#if proj.description}<p class="proj-desc">{loc(proj.description)}</p>{/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- Teaching -->
      {#if profile.teaching.length > 0 || isOwnProfile}
        <div class="sidebar-card">
          <div class="section-header">
            <h3>{t('profile.teaching')}</h3>
            {#if isOwnProfile}
              <button class="edit-section-btn" onclick={() => { editTeach = JSON.parse(JSON.stringify(profile!.teaching)); editingTeach = true; }}>
                {profile.teaching.length > 0 ? t('common.edit') : t('profile.add')}
              </button>
            {/if}
          </div>
          {#each profile.teaching.sort((a, b) => b.year - a.year) as te}
            <div class="teach-entry">
              <strong>{loc(te.course_name)}</strong>
              <span class="teach-meta">{loc(te.role)}{#if te.institution}, {loc(te.institution)}{/if}{#if te.year} ({te.year}){/if}</span>
              {#if te.description}<p class="teach-desc">{loc(te.description)}</p>{/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- Open Listings -->
      {#if userListings.length > 0}
        <div class="sidebar-card">
          <div class="section-header"><h3>{t('profile.openPositions')}</h3></div>
          {#each userListings as l}
            <a href="/listing?id={encodeURIComponent(l.id)}" class="listing-mini">
              <span class="listing-kind">{l.kind.toUpperCase()}</span>
              <span class="listing-title">{l.title}</span>
              <span class="listing-inst">{l.institution}</span>
            </a>
          {/each}
        </div>
      {/if}
    </aside>
  </div>
{/if}

<!-- Publications Editor Modal -->
{#if editingPubs}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => editingPubs = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal academic-modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('profile.editPublications')}</h3>
      {#each editPubs as pub_entry, i}
        <div class="modal-entry">
          <input type="text" bind:value={pub_entry.title[getLocale()]} placeholder={t('profile.pubTitle')} />
          <input type="text" bind:value={pub_entry.venue[getLocale()]} placeholder={t('profile.pubVenue')} />
          <div class="modal-row">
            <input type="text" value={pub_entry.authors.join(', ')} oninput={(e) => { pub_entry.authors = (e.target as HTMLInputElement).value.split(',').map(s => s.trim()); }} placeholder={t('profile.pubAuthors')} />
            <input type="number" bind:value={pub_entry.year} placeholder={t('profile.year')} class="year-input" />
          </div>
          <div class="modal-row">
            <input type="url" bind:value={pub_entry.url} placeholder="URL" />
            <input type="text" bind:value={pub_entry.doi} placeholder={t('profile.doi')} />
          </div>
          <button class="remove-entry" onclick={() => { editPubs = editPubs.filter((_, j) => j !== i); }}>{t('profile.remove')}</button>
        </div>
      {/each}
      <button class="add-entry" onclick={() => { editPubs = [...editPubs, { title: {}, authors: [], venue: {}, year: new Date().getFullYear(), url: null, doi: null, abstract_text: null }]; }}>+ {t('profile.addPublication')}</button>
      <div class="modal-actions">
        <button class="btn-cancel" onclick={() => editingPubs = false}>{t('common.cancel')}</button>
        <button class="btn-save" onclick={async () => { await updatePublications(editPubs); profile!.publications = editPubs; editingPubs = false; }}>{t('common.save')}</button>
      </div>
    </div>
  </div>
{/if}

<!-- Projects Editor Modal -->
{#if editingProjects}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => editingProjects = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal academic-modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('profile.editProjects')}</h3>
      {#each editProj as proj, i}
        <div class="modal-entry">
          <input type="text" bind:value={proj.title[getLocale()]} placeholder={t('profile.projectName')} />
          <textarea bind:value={proj.description[getLocale()]} placeholder={t('profile.description')} rows="2"></textarea>
          <div class="modal-row">
            <input type="url" bind:value={proj.url} placeholder="URL" />
            <select bind:value={proj.status}>
              <option value="active">{t('profile.projectStatus.active')}</option>
              <option value="completed">{t('profile.projectStatus.completed')}</option>
              <option value="archived">{t('profile.projectStatus.archived')}</option>
            </select>
          </div>
          <button class="remove-entry" onclick={() => { editProj = editProj.filter((_, j) => j !== i); }}>{t('profile.remove')}</button>
        </div>
      {/each}
      <button class="add-entry" onclick={() => { editProj = [...editProj, { title: {}, description: {}, url: null, status: 'active' }]; }}>+ {t('profile.addProject')}</button>
      <div class="modal-actions">
        <button class="btn-cancel" onclick={() => editingProjects = false}>{t('common.cancel')}</button>
        <button class="btn-save" onclick={async () => { await updateProjects(editProj); profile!.projects = editProj; editingProjects = false; }}>{t('common.save')}</button>
      </div>
    </div>
  </div>
{/if}

<!-- Teaching Editor Modal -->
{#if editingTeach}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => editingTeach = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal academic-modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('profile.editTeaching')}</h3>
      {#each editTeach as te, i}
        <div class="modal-entry">
          <input type="text" bind:value={te.course_name[getLocale()]} placeholder={t('profile.courseName')} />
          <div class="modal-row">
            <input type="text" bind:value={te.role[getLocale()]} placeholder={t('profile.courseRole')} />
            <input type="text" bind:value={te.institution[getLocale()]} placeholder={t('profile.institution')} />
            <input type="number" bind:value={te.year} placeholder={t('profile.year')} class="year-input" />
          </div>
          <button class="remove-entry" onclick={() => { editTeach = editTeach.filter((_, j) => j !== i); }}>{t('profile.remove')}</button>
        </div>
      {/each}
      <button class="add-entry" onclick={() => { editTeach = [...editTeach, { course_name: {}, role: {}, institution: {}, year: new Date().getFullYear(), description: null }]; }}>+ {t('profile.addCourse')}</button>
      <div class="modal-actions">
        <button class="btn-cancel" onclick={() => editingTeach = false}>{t('common.cancel')}</button>
        <button class="btn-save" onclick={async () => { await updateTeaching(editTeach); profile!.teaching = editTeach; editingTeach = false; }}>{t('common.save')}</button>
      </div>
    </div>
  </div>
{/if}

<!-- Education Editor Modal -->
{#if editingEdu}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => editingEdu = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal academic-modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('profile.editEducation')}</h3>
      {#each editEdu as edu, i}
        <div class="modal-entry">
          <div class="modal-row">
            <select bind:value={edu.degree}>
              <option value="">{t('profile.degreeSelect')}</option>
              <option value="Bachelor">{t('profile.degree.Bachelor')}</option>
              <option value="Master">{t('profile.degree.Master')}</option>
              <option value="PhD">{t('profile.degree.PhD')}</option>
              <option value="Postdoc">{t('profile.degree.Postdoc')}</option>
              <option value="Associate">{t('profile.degree.Associate')}</option>
              <option value="Other">{t('profile.degree.Other')}</option>
            </select>
            <select bind:value={edu.current} onchange={() => { if (edu.current) edu.end_date = null; }}>
              <option value={true}>{t('profile.enrolled')}</option>
              <option value={false}>{t('profile.graduated')}</option>
            </select>
          </div>
          <input type="text" bind:value={edu.school} placeholder={t('profile.school')} />
          <div class="modal-row">
            <input type="text" bind:value={edu.department} placeholder={t('profile.eduDepartment')} />
            <input type="text" bind:value={edu.major} placeholder={t('profile.major')} />
          </div>
          <div class="modal-row">
            <input type="month" bind:value={edu.start_date} placeholder={t('profile.start')} />
            <span class="date-sep">–</span>
            {#if edu.current}
              <span class="date-present">{t('profile.present')}</span>
            {:else}
              <input type="month" bind:value={edu.end_date} placeholder={t('profile.end')} />
            {/if}
          </div>
          <!-- Translations -->
          <details class="edu-translations">
            <summary class="edu-trans-summary">
              {t('profile.translations')} ({Object.keys(edu.translations || {}).length})
            </summary>
            {#each Object.keys(edu.translations || {}) as lang}
              {@const tr = (edu.translations || {})[lang]}
              <div class="edu-trans-block">
                <div class="edu-trans-header">
                  <span class="edu-trans-lang">{lang.toUpperCase()}</span>
                  <button class="remove-entry" onclick={() => { const copy = { ...(edu.translations || {}) }; delete copy[lang]; edu.translations = copy; editEdu = [...editEdu]; }}>×</button>
                </div>
                <input type="text" value={tr?.school || ''} placeholder={t('profile.school')} oninput={(e) => { if (!edu.translations) edu.translations = {}; if (!edu.translations[lang]) edu.translations[lang] = {}; edu.translations[lang].school = (e.target as HTMLInputElement).value || null; }} />
                <div class="modal-row">
                  <input type="text" value={tr?.department || ''} placeholder={t('profile.eduDepartment')} oninput={(e) => { if (!edu.translations) edu.translations = {}; if (!edu.translations[lang]) edu.translations[lang] = {}; edu.translations[lang].department = (e.target as HTMLInputElement).value || null; }} />
                  <input type="text" value={tr?.major || ''} placeholder={t('profile.major')} oninput={(e) => { if (!edu.translations) edu.translations = {}; if (!edu.translations[lang]) edu.translations[lang] = {}; edu.translations[lang].major = (e.target as HTMLInputElement).value || null; }} />
                </div>
              </div>
            {/each}
            <select class="edu-add-lang" onchange={(e) => { const v = (e.target as HTMLSelectElement).value; if (!v) return; if (!edu.translations) edu.translations = {}; edu.translations[v] = { school: null, department: null, major: null }; editEdu = [...editEdu]; (e.target as HTMLSelectElement).value = ''; }}>
              <option value="">{t('profile.addLanguage')}</option>
              {#each ['en', 'zh', 'ja', 'ko', 'fr', 'de'].filter(l => !Object.keys(edu.translations || {}).includes(l)) as l}
                <option value={l}>{l.toUpperCase()}</option>
              {/each}
            </select>
          </details>
          <button class="remove-entry" onclick={() => { editEdu = editEdu.filter((_, j) => j !== i); }}>{t('profile.remove')}</button>
        </div>
      {/each}
      <button class="add-entry" onclick={() => { editEdu = [...editEdu, { degree: '', school: '', department: '', major: '', start_date: '', end_date: '', current: true, translations: {} }]; }}>+ {t('profile.addEducation')}</button>
      <div class="modal-actions">
        <button class="btn-cancel" onclick={() => editingEdu = false}>{t('common.cancel')}</button>
        <button class="btn-save" onclick={async () => { await updateEducation(editEdu); profile!.education = editEdu; editingEdu = false; }}>{t('common.save')}</button>
      </div>
    </div>
  </div>
{/if}

<!-- Work Experience Editor Modal -->
{#if editingExp}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => editingExp = false}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal academic-modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('profile.editExperience')}</h3>
      {#each editExp as exp, i}
        <div class="modal-entry">
          <input type="text" bind:value={exp.company} placeholder={t('profile.company')} />
          <div class="modal-row">
            <input type="text" bind:value={exp.department} placeholder={t('profile.workDepartment')} />
            <input type="text" bind:value={exp.title} placeholder={t('profile.jobTitle')} />
            <input type="text" bind:value={exp.location} placeholder={t('profile.location')} />
          </div>
          <div class="modal-row">
            <input type="month" bind:value={exp.start_date} placeholder={t('profile.start')} />
            <span class="date-sep">–</span>
            {#if exp.current}
              <span class="date-present">{t('profile.present')}</span>
            {:else}
              <input type="month" bind:value={exp.end_date} placeholder={t('profile.end')} />
            {/if}
            <label class="current-toggle">
              <input type="checkbox" bind:checked={exp.current} onchange={() => { if (exp.current) exp.end_date = null; }} />
              {t('profile.enrolled')}
            </label>
          </div>
          <textarea bind:value={exp.description} placeholder={t('profile.description')} rows="2"></textarea>
          <!-- Translations -->
          <details class="edu-translations">
            <summary class="edu-trans-summary">
              {t('profile.translations')} ({Object.keys(exp.translations || {}).length})
            </summary>
            {#each Object.keys(exp.translations || {}) as lang}
              {@const tr = (exp.translations || {})[lang]}
              <div class="edu-trans-block">
                <div class="edu-trans-header">
                  <span class="edu-trans-lang">{lang.toUpperCase()}</span>
                  <button class="remove-entry" onclick={() => { const copy = { ...(exp.translations || {}) }; delete copy[lang]; exp.translations = copy; editExp = [...editExp]; }}>×</button>
                </div>
                <input type="text" value={tr?.company || ''} placeholder={t('profile.company')} oninput={(e) => { if (!exp.translations) exp.translations = {}; if (!exp.translations[lang]) exp.translations[lang] = {}; exp.translations[lang].company = (e.target as HTMLInputElement).value || null; }} />
                <div class="modal-row">
                  <input type="text" value={tr?.department || ''} placeholder={t('profile.workDepartment')} oninput={(e) => { if (!exp.translations) exp.translations = {}; if (!exp.translations[lang]) exp.translations[lang] = {}; exp.translations[lang].department = (e.target as HTMLInputElement).value || null; }} />
                  <input type="text" value={tr?.title || ''} placeholder={t('profile.jobTitle')} oninput={(e) => { if (!exp.translations) exp.translations = {}; if (!exp.translations[lang]) exp.translations[lang] = {}; exp.translations[lang].title = (e.target as HTMLInputElement).value || null; }} />
                  <input type="text" value={tr?.location || ''} placeholder={t('profile.location')} oninput={(e) => { if (!exp.translations) exp.translations = {}; if (!exp.translations[lang]) exp.translations[lang] = {}; exp.translations[lang].location = (e.target as HTMLInputElement).value || null; }} />
                </div>
                <textarea value={tr?.description || ''} placeholder={t('profile.description')} rows="2" oninput={(e) => { if (!exp.translations) exp.translations = {}; if (!exp.translations[lang]) exp.translations[lang] = {}; exp.translations[lang].description = (e.target as HTMLTextAreaElement).value || null; }}></textarea>
              </div>
            {/each}
            <select class="edu-add-lang" onchange={(e) => { const v = (e.target as HTMLSelectElement).value; if (!v) return; if (!exp.translations) exp.translations = {}; exp.translations[v] = {}; editExp = [...editExp]; (e.target as HTMLSelectElement).value = ''; }}>
              <option value="">{t('profile.addLanguage')}</option>
              {#each ['en', 'zh', 'ja', 'ko', 'fr', 'de'].filter(l => !Object.keys(exp.translations || {}).includes(l)) as l}
                <option value={l}>{l.toUpperCase()}</option>
              {/each}
            </select>
          </details>
          <button class="remove-entry" onclick={() => { editExp = editExp.filter((_, j) => j !== i); }}>{t('profile.remove')}</button>
        </div>
      {/each}
      <button class="add-entry" onclick={() => { editExp = [...editExp, { company: '', department: '', title: '', location: '', start_date: '', end_date: '', current: true, description: '', translations: {} }]; }}>+ {t('profile.addExperience')}</button>
      <div class="modal-actions">
        <button class="btn-cancel" onclick={() => editingExp = false}>{t('common.cancel')}</button>
        <button class="btn-save" onclick={async () => { await updateExperience(editExp); profile!.experience = editExp; editingExp = false; }}>{t('common.save')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .profile-header {
    position: relative;
    z-index: 1;
    display: flex;
    gap: 20px;
    align-items: flex-start;
    margin-top: 12px;
    margin-bottom: 24px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border);
  }
  /* Banner */
  .banner-wrap {
    position: relative;
    width: 100%;
    height: 180px;
    border-radius: 8px;
    overflow: hidden;
  }
  .banner-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .banner-placeholder {
    width: 100%;
    height: 100%;
    background: linear-gradient(135deg, rgba(95,155,101,0.15) 0%, rgba(95,155,101,0.05) 100%);
  }
  .banner-upload {
    position: absolute;
    bottom: 8px;
    right: 8px;
    padding: 4px 10px;
    background: rgba(255,255,255,0.85);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-hint);
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    transition: all 0.15s;
  }
  .banner-upload:hover { color: var(--accent); border-color: var(--accent); }

  .avatar-wrap { position: relative; }
  .avatar {
    width: 72px;
    height: 72px;
    border-radius: 50%;
    object-fit: cover;
  }
  .avatar-upload {
    position: absolute;
    bottom: 0;
    right: 0;
    width: 24px;
    height: 24px;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    color: var(--text-hint);
    transition: all 0.15s;
  }
  .avatar-upload:hover { color: var(--accent); border-color: var(--accent); }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0,0,0,0); }
  .avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-size: 28px;
    font-family: var(--font-serif);
  }
  .profile-info {
    flex: 1;
  }
  .follow-btn {
    padding: 6px 20px;
    font-size: 14px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    color: var(--accent);
    background: none;
    cursor: pointer;
    transition: all 0.15s;
    flex-shrink: 0;
    align-self: center;
  }
  .follow-btn:hover {
    background: var(--accent);
    color: white;
  }
  .follow-btn.following {
    background: var(--accent);
    color: white;
  }
  .follow-btn.following:hover {
    background: #dc2626;
    border-color: #dc2626;
  }
  .follow-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .display-name {
    font-family: var(--font-serif);
    margin: 0;
    font-size: 1.5rem;
  }
  .edit-name-btn {
    background: none; border: none; cursor: pointer; color: var(--text-hint);
    font-size: 14px; padding: 0 4px; vertical-align: middle;
  }
  .edit-name-btn:hover { color: var(--accent); }
  .name-edit { display: flex; gap: 8px; align-items: center; margin-bottom: 4px; }
  .name-input { font-size: 1.2rem; padding: 4px 8px; border: 1px solid var(--border); border-radius: 4px; font-family: var(--font-serif); }
  .handle {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 2px 0 0;
  }
  .rss-link {
    font-size: 11px;
    color: #f59e0b;
    background: rgba(245,158,11,0.1);
    padding: 2px 8px;
    border-radius: 3px;
    text-decoration: none;
    font-weight: 600;
  }
  .rss-link:hover { background: rgba(245,158,11,0.2); }
  .bio {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 6px 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .bio-placeholder {
    color: var(--text-hint);
    font-style: italic;
  }
  .bio-edit {
    margin: 6px 0;
  }
  .bio-input {
    width: 100%;
    font-size: 14px;
    line-height: 1.5;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-sans);
    color: var(--text-primary);
    background: var(--bg-white);
    resize: vertical;
  }
  .bio-edit-actions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .credential-line {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    color: var(--text-secondary);
    margin: 2px 0 0;
  }
  .verified-badge {
    display: inline-flex;
    align-items: center;
  }
  .education-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    margin-top: 2px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .education-entry {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 4px 0;
  }
  .edu-degree { font-weight: 600; font-size: 13px; color: var(--text-primary); }
  .edu-school { font-size: 13px; color: var(--text-secondary); }
  .edu-major { font-size: 12px; color: var(--text-hint); }
  .edu-dates { font-size: 12px; color: var(--text-hint); }
  .date-sep { color: var(--text-hint); line-height: 32px; }
  .date-present { font-size: 13px; color: var(--accent); line-height: 32px; }
  /* Education translation editor */
  .edu-translations { margin-top: 6px; }
  .edu-trans-summary { font-size: 12px; color: var(--text-hint); cursor: pointer; user-select: none; }
  .edu-trans-summary:hover { color: var(--accent); }
  .edu-trans-block { margin-top: 6px; padding: 6px 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-page, rgba(0,0,0,0.01)); }
  .edu-trans-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px; }
  .edu-trans-lang { font-size: 11px; font-weight: 600; color: var(--accent); text-transform: uppercase; }
  .edu-add-lang { font-size: 12px; margin-top: 6px; padding: 3px 6px; border: 1px dashed var(--border); border-radius: 3px; background: none; color: var(--text-hint); cursor: pointer; font-family: var(--font-sans); }

  /* Work experience */
  .experience-list { margin: 0; }
  .experience-entry { margin-bottom: 10px; font-size: 13px; line-height: 1.5; }
  .exp-company { font-weight: 600; display: block; }
  .exp-title { color: var(--text-secondary); display: block; }
  .exp-location { color: var(--text-hint); font-size: 12px; display: block; }
  .exp-desc { color: var(--text-secondary); font-size: 12px; margin: 4px 0 0; }
  .current-toggle { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; color: var(--text-secondary); cursor: pointer; }
  .profile-email {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    font-size: 13px;
  }
  .email-link {
    color: var(--text-secondary);
    text-decoration: none;
  }
  .email-link:hover { color: var(--accent); }
  .edit-email-btn {
    font-size: 12px;
    color: var(--text-hint);
    background: none;
    border: 1px dashed var(--border);
    border-radius: 3px;
    padding: 2px 8px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .edit-email-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .email-input {
    padding: 3px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-sans);
    width: 200px;
  }
  .email-save {
    font-size: 12px;
    padding: 3px 10px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .email-cancel {
    font-size: 12px;
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
  }
  .settings-link {
    font-size: 13px;
    color: var(--text-secondary);
    text-decoration: none;
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    transition: all 0.15s;
    align-self: center;
    flex-shrink: 0;
  }
  .settings-link:hover {
    border-color: var(--accent);
    color: var(--accent);
    text-decoration: none;
  }
  .profile-actions-secondary {
    display: flex;
    gap: 6px;
    align-self: center;
    flex-shrink: 0;
  }
  .action-btn {
    font-size: 12px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    color: var(--text-hint);
    cursor: pointer;
    transition: all 0.15s;
  }
  .action-btn:hover {
    border-color: var(--text-secondary);
    color: var(--text-secondary);
  }
  .action-btn.active {
    border-color: #dc2626;
    color: #dc2626;
  }
  .report-target {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0 0 10px;
  }
  .report-textarea {
    width: 100%;
    padding: 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-sans);
    resize: vertical;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .profile-stats {
    display: flex;
    gap: 16px;
    margin-top: 8px;
    font-size: 13px;
    color: var(--text-hint);
  }
  .rep-stat {
    color: var(--text-primary);
  }

  .section-title {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.1rem;
    margin: 0 0 12px;
    color: var(--text-secondary);
  }

  /* Card styles are now in PostCard.svelte */
  .empty-text { color: var(--text-hint); font-size: 14px; }
  .create-actions {
    display: flex;
    gap: 16px;
    margin-top: 12px;
  }
  .create-link {
    font-size: 14px;
    color: var(--accent);
  }

  /* Two-column layout inside the profile card: contacts left, education right. */
  .profile-columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr);
    gap: 24px;
    margin: 10px 0 16px;
  }
  @media (max-width: 800px) {
    .profile-columns { grid-template-columns: 1fr 1fr; }
  }
  @media (max-width: 540px) {
    .profile-columns { grid-template-columns: 1fr; gap: 12px; }
  }
  .profile-col { min-width: 0; }
  .col-heading {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-hint);
    margin: 0 0 6px;
  }

  /* Contacts list */
  .contacts-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .contact-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    text-decoration: none;
    padding: 2px 0;
    min-width: 0;
  }
  .contact-row:hover { color: var(--accent); text-decoration: none; }
  .contact-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    color: var(--text-hint);
    flex-shrink: 0;
  }
  .contact-row:hover .contact-icon { color: var(--accent); }
  .contact-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .contacts-edit-btn {
    align-self: flex-start;
    font-size: 12px;
    color: var(--text-hint);
    background: none;
    border: 1px dashed var(--border);
    border-radius: 3px;
    padding: 3px 10px;
    cursor: pointer;
    margin-top: 4px;
    transition: all 0.15s;
  }
  .contacts-edit-btn:hover { border-color: var(--accent); color: var(--accent); }

  /* Edit contacts modal layout */
  .contacts-edit-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    margin: 12px 0;
  }
  .contact-edit-row {
    display: grid;
    grid-template-columns: 20px 90px 1fr;
    align-items: center;
    gap: 8px;
  }
  .contact-edit-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-hint);
  }
  .contact-edit-label {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .contact-edit-input {
    font-size: 13px;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-input, var(--bg-white));
    color: var(--text-primary);
    min-width: 0;
  }
  .contact-edit-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .contact-edit-paired {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .contacts-custom-heading {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 14px 0 6px;
  }

  /* Bluesky badge next to handle */
  .handle-bsky {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-left: 6px;
    color: var(--text-hint);
    vertical-align: middle;
    text-decoration: none;
  }
  .handle-bsky:hover { color: #0085ff; }

  /* Profile actions row (follow / settings / RSS on one line) */
  .profile-actions-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
  }

  /* Edit links modal */
  .links-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.4);
    z-index: 400;
    display: flex;
    justify-content: center;
    padding-top: 10vh;
  }
  .links-modal {
    width: 480px;
    max-width: 90vw;
    background: var(--bg-white);
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.15);
    align-self: flex-start;
  }
  .links-modal h3 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .link-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
  }
  .link-label {
    font-weight: 500;
    font-size: 14px;
    min-width: 80px;
  }
  .link-url {
    font-size: 13px;
    color: var(--text-secondary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 18px;
    color: var(--text-hint);
    padding: 0 4px;
  }
  .link-remove:hover { color: #dc2626; }
  .link-add-row {
    display: flex;
    gap: 6px;
    margin-top: 10px;
  }
  .link-add-row input {
    flex: 1;
    padding: 5px 8px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: var(--font-sans);
  }
  .link-add-btn {
    padding: 5px 12px;
    font-size: 16px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }
  .link-add-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .link-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .link-cancel {
    padding: 5px 14px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: none;
    cursor: pointer;
  }
  .link-save {
    padding: 5px 14px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  /* Follow stats buttons */
  .stat-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-hint);
    padding: 0;
    transition: color 0.15s;
  }
  .stat-btn:hover { color: var(--accent); }
  .stat-btn strong { color: var(--text-secondary); }

  /* Follow list */
  .follow-list {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .follow-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    text-decoration: none;
    color: inherit;
    transition: opacity 0.15s;
  }
  .follow-item:hover { opacity: 0.8; text-decoration: none; }
  .follow-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }
  .follow-avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-family: var(--font-serif);
  }
  .follow-info {
    display: flex;
    flex-direction: column;
  }
  .follow-name {
    font-size: 14px;
    color: var(--text-primary);
  }
  .follow-handle {
    font-size: 12px;
    color: var(--text-hint);
  }

  .series-badge {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: var(--accent);
    background: rgba(95, 155, 101, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
    white-space: nowrap;
  }

  /* Series tree */
  .series-tree-node {
    margin-bottom: 8px;
  }
  .series-tree-card {
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 4px;
    padding: 12px 16px;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .series-tree-card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
  }
  .series-tree-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .expand-arrow {
    font-size: 10px;
    color: var(--text-hint);
    transition: transform 0.15s;
    flex-shrink: 0;
    width: 14px;
    text-align: center;
  }
  .expand-arrow.expanded {
    transform: rotate(90deg);
  }
  .expand-arrow-placeholder {
    width: 14px;
    flex-shrink: 0;
  }
  .series-tree-title {
    font-family: var(--font-serif);
    font-size: 1.1rem;
    color: var(--text-primary);
    text-decoration: none;
    flex: 1;
    min-width: 0;
  }
  .series-tree-title:hover {
    color: var(--accent);
    text-decoration: none;
  }
  .series-tree-desc {
    margin: 6px 0 0 22px;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .series-tree-bottom {
    margin-top: 8px;
    margin-left: 22px;
    display: flex;
    align-items: center;
  }

  /* Two-column body layout */
  .profile-body {
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 24px;
    align-items: start;
  }
  .profile-main {
    min-width: 0;
  }
  .profile-sidebar {
    display: flex;
    flex-direction: column;
    gap: 16px;
    position: sticky;
    top: 16px;
  }
  .sidebar-card {
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 16px;
  }
  .sidebar-card .section-header {
    margin-bottom: 10px;
  }
  .sidebar-card .section-header h3 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 0.95rem;
    margin: 0;
  }

  @media (max-width: 768px) {
    .profile-body {
      grid-template-columns: 1fr;
    }
    .profile-sidebar {
      position: static;
    }
  }

  /* Profile tabs */
  .profile-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 16px;
  }
  .tab-btn {
    padding: 8px 20px;
    font-size: 14px;
    font-family: var(--font-sans);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tab-btn:hover { color: var(--text-primary); }
  .tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .tab-count {
    font-size: 11px;
    background: var(--border);
    color: var(--text-secondary);
    padding: 1px 6px;
    border-radius: 8px;
  }

  /* Q&A cards in profile */
  .qa-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 6px;
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s;
  }
  .qa-card:hover {
    border-color: var(--border-strong);
    text-decoration: none;
  }
  .qa-card.question {
    border-left: 3px solid #d97706;
  }
  .qa-card.answer {
    border-left: 3px solid var(--accent);
  }
  .qa-badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .question-badge {
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
  }
  .answer-badge {
    color: var(--accent);
    background: rgba(95, 155, 101, 0.1);
  }
  .qa-title {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .qa-stat {
    font-size: 12px;
    color: var(--text-hint);
    flex-shrink: 0;
  }

  /* Bookmark cards */
  .bookmark-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 4px;
    margin-bottom: 6px;
    text-decoration: none;
    color: inherit;
    transition: border-color 0.15s;
  }
  .bookmark-card:hover {
    border-color: var(--border-strong);
    text-decoration: none;
  }
  .bookmark-info {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }
  .bookmark-title {
    font-size: 14px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bookmark-folder {
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-dim);
    padding: 1px 6px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .bookmark-date {
    font-size: 12px;
    color: var(--text-hint);
    flex-shrink: 0;
    margin-left: 12px;
  }

  /* ── 全部文章 tab ── */
  .all-series-group {
    margin-bottom: 20px;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .all-series-title {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--bg-dim, var(--bg));
    font-family: var(--font-serif);
    font-size: 1rem;
    color: var(--text-primary);
    text-decoration: none;
    border-bottom: 1px solid var(--border);
  }
  .all-series-title:hover { color: var(--accent); }
  .all-series-count {
    font-size: 12px;
    color: var(--text-hint);
    font-family: var(--font-sans);
    margin-left: auto;
  }
  .all-series-articles {
    display: flex;
    flex-direction: column;
  }
  .all-article-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 14px;
    border-bottom: 1px solid var(--border);
    text-decoration: none;
    color: var(--text-primary);
    font-size: 0.9rem;
  }
  .all-article-row:last-child { border-bottom: none; }
  .all-article-row:hover { background: var(--bg-hover, rgba(0,0,0,.03)); color: var(--accent); }
  .all-article-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .all-article-lang {
    font-size: 11px;
    color: var(--text-hint);
    background: var(--bg-dim);
    padding: 1px 5px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .section-header h3 { font-family: var(--font-serif); font-weight: 400; font-size: 1rem; margin: 0; }
  .edit-section-btn { font-size: 12px; color: var(--accent); background: none; border: none; cursor: pointer; }

  .pub-entry { font-size: 13px; margin-bottom: 6px; line-height: 1.5; }
  .pub-authors { color: var(--text-secondary); }
  .pub-title { color: var(--text-primary); font-style: italic; }
  a.pub-title { color: var(--accent); text-decoration: none; }
  a.pub-title:hover { text-decoration: underline; }
  .pub-venue { color: var(--text-secondary); font-weight: 500; }
  .pub-year { color: var(--text-hint); }

  .proj-entry { margin-bottom: 8px; }
  .proj-top { display: flex; align-items: center; gap: 8px; }
  .proj-title { font-size: 14px; font-weight: 500; color: var(--text-primary); }
  a.proj-title { color: var(--accent); text-decoration: none; }
  a.proj-title:hover { text-decoration: underline; }
  .proj-desc { font-size: 13px; color: var(--text-secondary); margin: 2px 0 0; }
  .status-badge { font-size: 10px; padding: 1px 6px; border-radius: 3px; }
  .status-active { background: rgba(16,185,129,0.1); color: #059669; }
  .status-completed { background: rgba(99,102,241,0.1); color: #4f46e5; }
  .status-archived { background: var(--bg-page); color: var(--text-hint); }

  .teach-entry { font-size: 13px; margin-bottom: 6px; }
  .teach-meta { color: var(--text-secondary); margin-left: 6px; }
  .teach-desc { font-size: 12px; color: var(--text-hint); margin: 2px 0 0; }

  .listing-mini { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: 4px; text-decoration: none; transition: background 0.1s; }
  .listing-mini:hover { background: var(--bg-hover); text-decoration: none; }
  .listing-kind { font-size: 10px; font-weight: 600; text-transform: uppercase; padding: 2px 6px; border-radius: 3px; background: var(--bg-page); border: 1px solid var(--border); color: var(--text-secondary); flex-shrink: 0; }
  .listing-title { font-size: 13px; color: var(--text-primary); }
  .listing-inst { font-size: 12px; color: var(--text-hint); margin-left: auto; }

  /* Academic edit modals */
  .modal-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.3); z-index: 300; display: flex; justify-content: center; padding-top: 8vh; }
  .academic-modal { width: 560px; max-width: 90vw; max-height: 80vh; overflow-y: auto; background: var(--bg-white); border-radius: 8px; padding: 24px; box-shadow: 0 8px 32px rgba(0,0,0,0.15); }
  .academic-modal h3 { font-family: var(--font-serif); font-weight: 400; margin: 0 0 16px; }
  .modal-entry { padding: 10px; margin-bottom: 8px; border: 1px solid var(--border); border-radius: 6px; }
  .modal-entry input, .modal-entry textarea, .modal-entry select { display: block; width: 100%; margin-top: 4px; padding: 6px 8px; font-size: 13px; border: 1px solid var(--border); border-radius: 3px; font-family: var(--font-sans); }
  .modal-entry textarea { resize: vertical; }
  .modal-row { display: flex; gap: 6px; margin-top: 4px; }
  .modal-row input, .modal-row select { flex: 1; }
  .year-input { max-width: 80px; }
  .remove-entry { font-size: 11px; color: #dc2626; background: none; border: none; cursor: pointer; margin-top: 4px; }
  .checkbox-label { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 400; color: var(--text-secondary); margin-top: 4px; cursor: pointer; }
  .add-entry { font-size: 13px; color: var(--accent); background: none; border: 1px dashed var(--accent); border-radius: 4px; padding: 6px 12px; cursor: pointer; width: 100%; margin: 8px 0; }
  .modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 12px; }
  .btn-cancel { padding: 6px 14px; font-size: 13px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; }
  .btn-save { padding: 6px 14px; font-size: 13px; border: none; border-radius: 3px; background: var(--accent); color: white; cursor: pointer; }
</style>
