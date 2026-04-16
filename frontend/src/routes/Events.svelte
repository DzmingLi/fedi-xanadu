<script lang="ts">
  import { listEvents } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { fmtRep } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { NbEvent, EventKind } from '../lib/types';

  const KIND_KEYS: Record<EventKind, string> = {
    conference: 'events.kind.conference',
    workshop: 'events.kind.workshop',
    seminar: 'events.kind.seminar',
    meetup: 'events.kind.meetup',
    hackathon: 'events.kind.hackathon',
  };

  const KIND_COLORS: Record<EventKind, string> = {
    conference: '#7c3aed',
    workshop: '#059669',
    seminar: '#2563eb',
    meetup: '#d97706',
    hackathon: '#dc2626',
  };

  const ALL_KINDS: EventKind[] = ['conference', 'workshop', 'seminar', 'meetup', 'hackathon'];

  let events = $state<NbEvent[]>([]);
  let loading = $state(true);
  let tab = $state<'upcoming' | 'past' | 'all'>('upcoming');
  let kindFilter = $state<string>('');
  let isLoggedIn = $derived(!!getAuth());

  let displayedEvents = $derived.by(() => {
    let list = events;
    const now = new Date().toISOString();
    if (tab === 'upcoming') list = list.filter(e => e.start_time >= now);
    else if (tab === 'past') list = list.filter(e => e.start_time < now);
    if (kindFilter) list = list.filter(e => e.kind === kindFilter);
    return list;
  });

  async function load() {
    loading = true;
    try {
      events = await listEvents(kindFilter || undefined, undefined, tab === 'upcoming' ? true : tab === 'past' ? false : undefined);
    } catch { /* */ }
    loading = false;
  }

  $effect(() => {
    document.title = `${t('events.title')} — NightBoat`;
    load();
  });

  function fmtDateTime(iso: string): string {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }) +
      ' ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }
</script>

<div class="events-page">
  <div class="page-header">
    <h1>{t('events.title')}</h1>
    {#if isLoggedIn}
      <a href="/new-event" class="btn-new">{t('events.create')}</a>
    {/if}
  </div>

  <div class="category-tabs">
    <button class="cat-tab" class:active={tab === 'upcoming'} onclick={() => { tab = 'upcoming'; }}>{t('events.upcoming')}</button>
    <button class="cat-tab" class:active={tab === 'past'} onclick={() => { tab = 'past'; }}>{t('events.past')}</button>
    <button class="cat-tab" class:active={tab === 'all'} onclick={() => { tab = 'all'; }}>{t('events.all')}</button>
  </div>

  <div class="filters">
    <button class:active={kindFilter === ''} onclick={() => kindFilter = ''}>{t('events.all')}</button>
    {#each ALL_KINDS as k}
      <button class:active={kindFilter === k} onclick={() => kindFilter = k}>{t(KIND_KEYS[k])}</button>
    {/each}
  </div>

  {#if loading}
    <div class="empty">Loading...</div>
  {:else if displayedEvents.length === 0}
    <div class="empty">{t('events.empty')}</div>
  {:else}
    <div class="event-list">
      {#each displayedEvents as ev}
        <a href="/event?id={encodeURIComponent(ev.id)}" class="event-card" class:cancelled={ev.is_cancelled}>
          <div class="card-top">
            <span class="kind-badge" style="background: {KIND_COLORS[ev.kind]}20; color: {KIND_COLORS[ev.kind]}; border-color: {KIND_COLORS[ev.kind]}40">{t(KIND_KEYS[ev.kind])}</span>
            {#if ev.is_cancelled}<span class="cancelled-badge">{t('events.cancelled')}</span>{/if}
            {#if ev.max_attendees && ev.rsvp_count >= ev.max_attendees}<span class="full-badge">{t('events.full')}</span>{/if}
          </div>
          <h3>{ev.title}</h3>
          <div class="card-meta">
            <span class="organizer">{ev.organizer}</span>
            <span class="dot">·</span>
            <span>{ev.location || t('events.online')}</span>
          </div>
          <div class="card-time">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
            <span>{fmtDateTime(ev.start_time)}</span>
            {#if ev.end_time}<span> — {fmtDateTime(ev.end_time)}</span>{/if}
          </div>
          <div class="card-bottom">
            <span class="rsvp-count">{t('events.rsvpCount').replace('{0}', String(ev.rsvp_count))}</span>
            {#if ev.teaches.length > 0}
              <div class="tags">
                {#each ev.teaches as tag}
                  <span class="tag">{tag}</span>
                {/each}
              </div>
            {/if}
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>

<style>
  .events-page { max-width: 760px; margin: 0 auto; }
  .page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; }
  .page-header h1 { font-family: var(--font-serif); font-weight: 400; margin: 0; }
  .btn-new { font-size: 13px; padding: 6px 14px; border: 1px solid var(--accent); border-radius: 3px; color: var(--accent); text-decoration: none; transition: all 0.15s; }
  .btn-new:hover { background: var(--accent); color: white; text-decoration: none; }

  .category-tabs { display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: 12px; }
  .cat-tab { padding: 8px 16px; font-size: 14px; font-weight: 500; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-secondary); cursor: pointer; font-family: var(--font-serif); }
  .cat-tab.active { color: var(--text-primary); border-bottom-color: var(--accent); }

  .filters { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 1.5rem; }
  .filters button { padding: 4px 12px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; }
  .filters button.active { background: var(--accent); color: white; border-color: var(--accent); }

  .event-list { display: flex; flex-direction: column; gap: 8px; }
  .event-card { display: block; padding: 14px 16px; border: 1px solid var(--border); border-radius: 6px; text-decoration: none; color: var(--text-primary); transition: border-color 0.15s; }
  .event-card:hover { border-color: var(--accent); text-decoration: none; }
  .event-card.cancelled { opacity: 0.6; }

  .card-top { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .kind-badge { font-size: 11px; font-weight: 600; text-transform: uppercase; padding: 2px 6px; border-radius: 3px; border: 1px solid; }
  .cancelled-badge { font-size: 11px; font-weight: 600; padding: 2px 6px; border-radius: 3px; background: #fef2f2; color: #dc2626; }
  .full-badge { font-size: 11px; font-weight: 600; padding: 2px 6px; border-radius: 3px; background: #fef3c7; color: #92400e; }

  .event-card h3 { margin: 0 0 4px; font-size: 15px; font-family: var(--font-serif); font-weight: 400; }

  .card-meta { font-size: 13px; color: var(--text-secondary); display: flex; flex-wrap: wrap; gap: 0 4px; margin-bottom: 4px; }
  .organizer { font-weight: 500; }
  .dot { color: var(--text-hint); }

  .card-time { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--text-hint); margin-bottom: 6px; }

  .card-bottom { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .rsvp-count { font-size: 12px; color: var(--text-hint); }

  .tags { display: flex; flex-wrap: wrap; gap: 4px; }
  .tag { font-size: 11px; padding: 2px 6px; border-radius: 3px; background: #e0e7ff; color: #3730a3; }

  .empty { padding: 2rem; text-align: center; color: var(--text-hint); font-size: 14px; }
</style>
