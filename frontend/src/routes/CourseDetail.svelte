<script lang="ts">
  import { getCourse, createUnit, createItem, deleteUnit, deleteItem, updateCourse, deleteCourse } from '../lib/api';
  import { t, getLocale, onLocaleChange } from '../lib/i18n';
  import { getAuth } from '../lib/auth';
  import { toast } from '../lib/components/Toast.svelte';
  import type { CourseDetail } from '../lib/types';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let { id }: { id: string } = $props();
  let detail = $state<CourseDetail | null>(null);
  let loading = $state(true);
  let isOwner = $derived(detail && getAuth()?.did === detail.course.instructor_did);

  // New unit form
  let showNewUnit = $state(false);
  let newUnitTitle = $state('');
  let newUnitDesc = $state('');

  // New item forms per unit
  let addingItemUnit = $state<string | null>(null);
  let newItemTitle = $state('');
  let newItemRole = $state('reading');
  let newItemUrl = $state('');
  let newItemNote = $state('');

  $effect(() => {
    if (id) load();
  });

  async function load() {
    loading = true;
    try {
      detail = await getCourse(id);
    } catch (e: any) {
      toast(e.message, 'error');
    }
    loading = false;
  }

  async function handleAddUnit() {
    if (!newUnitTitle.trim()) return;
    try {
      await createUnit(id, newUnitTitle.trim(), newUnitDesc.trim() || undefined);
      newUnitTitle = '';
      newUnitDesc = '';
      showNewUnit = false;
      await load();
    } catch (e: any) {
      toast(e.message, 'error');
    }
  }

  async function handleAddItem(unitId: string) {
    if (!newItemTitle.trim()) return;
    try {
      await createItem(unitId, newItemTitle.trim(), {
        role: newItemRole,
        external_url: newItemUrl.trim() || undefined,
        note: newItemNote.trim() || undefined,
      });
      newItemTitle = '';
      newItemRole = 'reading';
      newItemUrl = '';
      newItemNote = '';
      addingItemUnit = null;
      await load();
    } catch (e: any) {
      toast(e.message, 'error');
    }
  }

  async function handleDeleteUnit(unitId: string) {
    if (!confirm(t('courses.confirmDeleteUnit'))) return;
    try {
      await deleteUnit(unitId);
      await load();
    } catch (e: any) { toast(e.message, 'error'); }
  }

  async function handleDeleteItem(itemId: string) {
    if (!confirm(t('courses.confirmDeleteItem'))) return;
    try {
      await deleteItem(itemId);
      await load();
    } catch (e: any) { toast(e.message, 'error'); }
  }

  async function handleDeleteCourse() {
    if (!confirm(t('courses.confirmDeleteCourse'))) return;
    try {
      await deleteCourse(id);
      window.location.hash = '#/courses';
    } catch (e: any) { toast(e.message, 'error'); }
  }

  const roleIcon = (role: string) => {
    switch (role) {
      case 'reading': return '📖';
      case 'video': return '🎬';
      case 'assignment': return '📝';
      case 'supplement': return '📎';
      case 'lecture': return '🎓';
      default: return '📄';
    }
  };

  const roleLabel = (role: string) => {
    const key = `courses.role.${role}`;
    return t(key) || role;
  };
</script>

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if !detail}
  <p class="meta">Not found</p>
{:else}
  <div class="course-header">
    {#if detail.course.cover_url}
      <img src={detail.course.cover_url} alt="" class="cover" />
    {/if}
    <div class="header-text">
      <h1>{detail.course.title}</h1>
      {#if detail.course.description}
        <p class="desc">{detail.course.description}</p>
      {/if}
      <div class="meta-row">
        <span class="schedule-badge">{detail.course.schedule_type}</span>
        {#if detail.course.term || detail.course.year}
          <span class="term-badge">{detail.course.term ?? ''} {detail.course.year ?? ''}</span>
        {/if}
        <span class="date">{new Date(detail.course.created_at).toLocaleDateString()}</span>
      </div>
    </div>
    {#if isOwner}
      <div class="owner-actions">
        <a href="#/new-course?canonical_id={encodeURIComponent(detail.course.canonical_id || detail.course.id)}&canonical_title={encodeURIComponent(detail.course.title)}" class="btn-outline">{t('courses.newOffering')}</a>
        <button class="btn-danger" onclick={handleDeleteCourse}>{t('common.delete')}</button>
      </div>
    {/if}
  </div>

  <!-- Other offerings -->
  {#if detail.offerings.length > 0}
    <div class="offerings">
      <h3>{t('courses.otherOfferings')}</h3>
      <div class="offering-list">
        {#each detail.offerings as off}
          <a href="#/course?id={encodeURIComponent(off.id)}" class="offering-chip" class:current={off.id === id}>
            {off.term ?? ''} {off.year ?? ''}{!off.term && !off.year ? off.title : ''}
          </a>
        {/each}
        <span class="offering-chip current">
          {detail.course.term ?? ''} {detail.course.year ?? ''}{!detail.course.term && !detail.course.year ? t('courses.current') : ''} ({t('courses.current')})
        </span>
      </div>
    </div>
  {/if}

  <!-- Units -->
  <div class="units">
    {#if detail.units.length === 0 && !isOwner}
      <p class="empty">{t('courses.noUnits')}</p>
    {/if}

    {#each detail.units as { unit, items }, ui}
      <div class="unit">
        <div class="unit-header">
          <h2 class="unit-title">
            <span class="unit-num">{ui + 1}.</span>
            {unit.title}
          </h2>
          {#if unit.available_from}
            <span class="unit-date">{unit.available_from}</span>
          {/if}
          {#if isOwner}
            <button class="btn-icon" onclick={() => handleDeleteUnit(unit.id)} title={t('common.delete')}>✕</button>
          {/if}
        </div>
        {#if unit.description}
          <p class="unit-desc">{unit.description}</p>
        {/if}

        <div class="items">
          {#each items as item}
            <div class="item">
              <span class="item-icon">{roleIcon(item.role)}</span>
              <div class="item-body">
                <div class="item-top">
                  <span class="item-role-badge">{roleLabel(item.role)}</span>
                  {#if item.external_url}
                    <a href={item.external_url} target="_blank" rel="noopener" class="item-title-link">{item.title}</a>
                  {:else if item.target_uri}
                    <a href="#/article?uri={encodeURIComponent(item.target_uri)}" class="item-title-link">{item.title}</a>
                  {:else}
                    <span class="item-title">{item.title}</span>
                  {/if}
                  {#if item.due_date}
                    <span class="item-due">Due: {item.due_date}</span>
                  {/if}
                </div>
                {#if item.note}
                  <p class="item-note">{item.note}</p>
                {/if}
              </div>
              {#if isOwner}
                <button class="btn-icon btn-sm" onclick={() => handleDeleteItem(item.id)} title={t('common.delete')}>✕</button>
              {/if}
            </div>
          {/each}
        </div>

        {#if isOwner}
          {#if addingItemUnit === unit.id}
            <form class="add-item-form" onsubmit={(e) => { e.preventDefault(); handleAddItem(unit.id); }}>
              <input type="text" bind:value={newItemTitle} placeholder={t('courses.itemTitle')} required />
              <select bind:value={newItemRole}>
                <option value="reading">{t('courses.role.reading')}</option>
                <option value="video">{t('courses.role.video')}</option>
                <option value="assignment">{t('courses.role.assignment')}</option>
                <option value="supplement">{t('courses.role.supplement')}</option>
                <option value="lecture">{t('courses.role.lecture')}</option>
              </select>
              <input type="url" bind:value={newItemUrl} placeholder={t('courses.externalUrl')} />
              <input type="text" bind:value={newItemNote} placeholder={t('courses.note')} />
              <div class="form-btns">
                <button type="submit" class="btn-sm-primary">{t('common.add')}</button>
                <button type="button" class="btn-sm-secondary" onclick={() => { addingItemUnit = null; }}>{t('common.cancel')}</button>
              </div>
            </form>
          {:else}
            <button class="add-item-btn" onclick={() => { addingItemUnit = unit.id; newItemTitle = ''; newItemUrl = ''; newItemNote = ''; newItemRole = 'reading'; }}>
              + {t('courses.addItem')}
            </button>
          {/if}
        {/if}
      </div>
    {/each}

    {#if isOwner}
      {#if showNewUnit}
        <form class="add-unit-form" onsubmit={(e) => { e.preventDefault(); handleAddUnit(); }}>
          <input type="text" bind:value={newUnitTitle} placeholder={t('courses.unitTitle')} required />
          <input type="text" bind:value={newUnitDesc} placeholder={t('courses.description')} />
          <div class="form-btns">
            <button type="submit" class="btn-sm-primary">{t('common.add')}</button>
            <button type="button" class="btn-sm-secondary" onclick={() => { showNewUnit = false; }}>{t('common.cancel')}</button>
          </div>
        </form>
      {:else}
        <button class="add-unit-btn" onclick={() => { showNewUnit = true; newUnitTitle = ''; newUnitDesc = ''; }}>
          + {t('courses.addUnit')}
        </button>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .course-header {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    margin-bottom: 24px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .cover {
    width: 120px;
    height: 120px;
    object-fit: cover;
    border-radius: 6px;
    flex-shrink: 0;
  }
  .header-text { flex: 1; min-width: 0; }
  .header-text h1 {
    margin: 0 0 6px;
    font-size: 22px;
  }
  .desc {
    margin: 0 0 8px;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.5;
  }
  .meta-row {
    display: flex;
    gap: 10px;
    align-items: center;
    font-size: 12px;
    color: var(--text-hint);
  }
  .schedule-badge {
    background: var(--bg-light);
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11px;
  }
  .owner-actions { flex-shrink: 0; display: flex; gap: 8px; align-items: flex-start; }
  .btn-outline {
    padding: 4px 12px;
    font-size: 12px;
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 4px;
    text-decoration: none;
    transition: all 0.15s;
  }
  .btn-outline:hover { background: var(--accent); color: white; text-decoration: none; }
  .term-badge {
    background: rgba(59, 130, 246, 0.1);
    color: var(--accent);
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 500;
  }

  /* Offerings */
  .offerings {
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .offerings h3 {
    margin: 0 0 8px;
    font-size: 14px;
    color: var(--text-secondary);
    font-weight: 500;
  }
  .offering-list { display: flex; gap: 6px; flex-wrap: wrap; }
  .offering-chip {
    padding: 4px 10px;
    font-size: 12px;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--accent);
    text-decoration: none;
    transition: all 0.15s;
  }
  .offering-chip:hover { background: var(--bg-light); text-decoration: none; }
  .offering-chip.current {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    pointer-events: none;
  }

  /* Units */
  .units { display: flex; flex-direction: column; gap: 20px; }
  .unit {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    background: var(--bg-white);
  }
  .unit-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }
  .unit-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    flex: 1;
  }
  .unit-num {
    color: var(--text-hint);
    font-weight: 400;
    margin-right: 4px;
  }
  .unit-date {
    font-size: 12px;
    color: var(--text-hint);
  }
  .unit-desc {
    margin: 0 0 10px;
    font-size: 13px;
    color: var(--text-secondary);
  }

  /* Items */
  .items { display: flex; flex-direction: column; gap: 6px; }
  .item {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 8px 10px;
    border-radius: 4px;
    background: var(--bg-light);
    transition: background 0.1s;
  }
  .item:hover { background: var(--bg-hover); }
  .item-icon { font-size: 16px; flex-shrink: 0; margin-top: 1px; }
  .item-body { flex: 1; min-width: 0; }
  .item-top {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .item-role-badge {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--border);
    color: var(--text-hint);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .item-title-link {
    font-size: 14px;
    color: var(--accent);
    text-decoration: none;
  }
  .item-title-link:hover { text-decoration: underline; }
  .item-title {
    font-size: 14px;
    color: var(--text-primary);
  }
  .item-due {
    font-size: 11px;
    color: var(--text-hint);
    margin-left: auto;
  }
  .item-note {
    margin: 3px 0 0;
    font-size: 12px;
    color: var(--text-hint);
    line-height: 1.4;
  }

  /* Buttons */
  .btn-icon {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    color: var(--text-hint);
    padding: 2px 4px;
    border-radius: 3px;
    transition: color 0.1s, background 0.1s;
  }
  .btn-icon:hover {
    color: var(--text-danger, #e53e3e);
    background: rgba(229, 62, 62, 0.08);
  }
  .btn-sm { font-size: 12px; }
  .btn-danger {
    padding: 4px 12px;
    font-size: 12px;
    color: white;
    background: var(--text-danger, #e53e3e);
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .add-unit-btn, .add-item-btn {
    padding: 6px 12px;
    font-size: 13px;
    color: var(--accent);
    background: none;
    border: 1px dashed var(--accent);
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s;
    margin-top: 8px;
  }
  .add-unit-btn:hover, .add-item-btn:hover {
    background: rgba(59, 130, 246, 0.05);
  }

  /* Forms */
  .add-unit-form, .add-item-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 10px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-light);
  }
  .add-unit-form input, .add-item-form input, .add-item-form select {
    font-size: 13px;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  .form-btns {
    display: flex;
    gap: 8px;
  }
  .btn-sm-primary {
    padding: 4px 14px;
    font-size: 12px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .btn-sm-secondary {
    padding: 4px 14px;
    font-size: 12px;
    background: none;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }

  .empty { color: var(--text-hint); font-size: 14px; }
  .meta { color: var(--text-hint); font-size: 14px; }
</style>
