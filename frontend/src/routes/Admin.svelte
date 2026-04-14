<script lang="ts">
  import {
    adminListReports, adminResolveReport,
    adminListAppeals, adminResolveAppeal,
    adminListBannedUsers, adminBanUser, adminUnbanUser,
    adminListUsers, adminDeleteArticle,
    type AdminReport, type AdminAppeal, type AdminBannedUser, type AdminPlatformUser,
  } from '../lib/api';
  import { t } from '../lib/i18n/index.svelte';

  // Admin auth — stored in sessionStorage (not localStorage for security)
  let secret = $state(sessionStorage.getItem('admin_secret') || '');
  let authenticated = $state(false);
  let authError = $state('');

  // Data
  let reports = $state<AdminReport[]>([]);
  let appeals = $state<AdminAppeal[]>([]);
  let bannedUsers = $state<AdminBannedUser[]>([]);
  let users = $state<AdminPlatformUser[]>([]);
  let loading = $state(false);

  // UI state
  let tab = $state<'reports' | 'appeals' | 'bans' | 'users'>('reports');
  let reportFilter = $state<'pending' | 'all'>('pending');

  // Action modals
  let resolveTarget = $state<{ type: 'report' | 'appeal'; id: string } | null>(null);
  let resolveStatus = $state('');
  let resolveNote = $state('');
  let banTarget = $state('');
  let banReason = $state('');
  let deleteUri = $state('');
  let deleteReason = $state('');

  async function doAuth() {
    authError = '';
    try {
      await adminListReports(secret, 'pending');
      authenticated = true;
      sessionStorage.setItem('admin_secret', secret);
      loadAll();
    } catch {
      authError = 'Invalid admin secret';
    }
  }

  async function loadAll() {
    loading = true;
    try {
      const [r, a, b, u] = await Promise.all([
        adminListReports(secret, reportFilter === 'pending' ? 'pending' : undefined),
        adminListAppeals(secret),
        adminListBannedUsers(secret),
        adminListUsers(secret),
      ]);
      reports = r;
      appeals = a;
      bannedUsers = b;
      users = u;
    } catch { /* */ }
    loading = false;
  }

  async function loadReports() {
    try {
      reports = await adminListReports(secret, reportFilter === 'pending' ? 'pending' : undefined);
    } catch { /* */ }
  }

  function openResolve(type: 'report' | 'appeal', id: string) {
    resolveTarget = { type, id };
    resolveStatus = '';
    resolveNote = '';
  }

  async function submitResolve() {
    if (!resolveTarget || !resolveStatus) return;
    try {
      if (resolveTarget.type === 'report') {
        await adminResolveReport(secret, resolveTarget.id, resolveStatus, resolveNote || undefined);
      } else {
        await adminResolveAppeal(secret, resolveTarget.id, resolveStatus, resolveNote || undefined);
      }
      resolveTarget = null;
      loadAll();
    } catch { /* */ }
  }

  async function doBan() {
    if (!banTarget) return;
    try {
      await adminBanUser(secret, banTarget, banReason || undefined);
      banTarget = '';
      banReason = '';
      loadAll();
    } catch { /* */ }
  }

  async function doUnban(did: string) {
    if (!confirm(`Unban ${did}?`)) return;
    try {
      await adminUnbanUser(secret, did);
      loadAll();
    } catch { /* */ }
  }

  async function doDelete() {
    if (!deleteUri) return;
    try {
      await adminDeleteArticle(secret, deleteUri, deleteReason || undefined);
      deleteUri = '';
      deleteReason = '';
    } catch { /* */ }
  }

  function fmtDate(s: string | null) {
    if (!s) return '—';
    return new Date(s).toLocaleString();
  }

  function pendingCount(items: { status: string }[]) {
    return items.filter(i => i.status === 'pending').length;
  }
</script>

{#if !authenticated}
  <div class="auth-box">
    <h1>Admin Dashboard</h1>
    <label>
      Admin Secret
      <input type="password" bind:value={secret} onkeydown={(e) => { if (e.key === 'Enter') doAuth(); }} placeholder="Enter admin secret" />
    </label>
    {#if authError}
      <div class="error">{authError}</div>
    {/if}
    <button class="btn-primary" onclick={doAuth}>Authenticate</button>
  </div>
{:else}
  <div class="admin">
    <div class="admin-header">
      <h1>Admin Dashboard</h1>
      <button class="btn-refresh" onclick={loadAll} disabled={loading}>
        {loading ? 'Loading...' : 'Refresh'}
      </button>
    </div>

    <nav class="admin-tabs">
      <button class:active={tab === 'reports'} onclick={() => tab = 'reports'}>
        Reports
        {#if pendingCount(reports) > 0}
          <span class="badge">{pendingCount(reports)}</span>
        {/if}
      </button>
      <button class:active={tab === 'appeals'} onclick={() => tab = 'appeals'}>
        Appeals
        {#if appeals.length > 0}
          <span class="badge">{appeals.length}</span>
        {/if}
      </button>
      <button class:active={tab === 'bans'} onclick={() => tab = 'bans'}>
        Bans
        {#if bannedUsers.length > 0}
          <span class="badge-muted">{bannedUsers.length}</span>
        {/if}
      </button>
      <button class:active={tab === 'users'} onclick={() => tab = 'users'}>
        Users
        <span class="badge-muted">{users.length}</span>
      </button>
    </nav>

    <!-- Reports -->
    {#if tab === 'reports'}
      <div class="tab-bar">
        <button class:active={reportFilter === 'pending'} onclick={() => { reportFilter = 'pending'; loadReports(); }}>Pending</button>
        <button class:active={reportFilter === 'all'} onclick={() => { reportFilter = 'all'; loadReports(); }}>All</button>
      </div>

      {#if reports.length === 0}
        <div class="empty">No reports</div>
      {:else}
        <div class="card-list">
          {#each reports as r}
            <div class="card" class:resolved={r.status !== 'pending'}>
              <div class="card-header">
                <span class="kind-badge">{r.kind}</span>
                <span class="status-badge" class:pending={r.status === 'pending'} class:dismissed={r.status === 'dismissed'}>{r.status}</span>
                <span class="date">{fmtDate(r.created_at)}</span>
              </div>
              <div class="card-body">
                <div class="field">
                  <strong>Reporter:</strong> @{r.reporter_handle || r.reporter_did}
                </div>
                <div class="field">
                  <strong>Target:</strong> @{r.target_handle || r.target_did}
                  {#if r.target_uri}
                    — <a href="/article?uri={encodeURIComponent(r.target_uri)}" target="_blank">View content</a>
                  {/if}
                </div>
                <div class="field reason">{r.reason}</div>
                {#if r.admin_note}
                  <div class="field admin-note"><strong>Admin note:</strong> {r.admin_note}</div>
                {/if}
              </div>
              {#if r.status === 'pending'}
                <div class="card-actions">
                  <button class="btn-action btn-resolve" onclick={() => openResolve('report', r.id)}>Resolve</button>
                  <button class="btn-action btn-dismiss" onclick={() => { adminResolveReport(secret, r.id, 'dismissed'); loadAll(); }}>Dismiss</button>
                  {#if r.target_uri}
                    <button class="btn-action btn-danger" onclick={() => { deleteUri = r.target_uri!; deleteReason = r.reason; }}>Delete Content</button>
                  {/if}
                  <button class="btn-action btn-danger" onclick={() => { banTarget = r.target_did; banReason = `Report: ${r.reason}`; }}>Ban User</button>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

    <!-- Appeals -->
    {:else if tab === 'appeals'}
      {#if appeals.length === 0}
        <div class="empty">No pending appeals</div>
      {:else}
        <div class="card-list">
          {#each appeals as a}
            <div class="card">
              <div class="card-header">
                <span class="kind-badge">{a.kind}</span>
                <span class="status-badge pending">{a.status}</span>
                <span class="date">{fmtDate(a.created_at)}</span>
              </div>
              <div class="card-body">
                <div class="field"><strong>User:</strong> {a.did}</div>
                {#if a.target_uri}
                  <div class="field"><strong>Article:</strong> <a href="/article?uri={encodeURIComponent(a.target_uri)}" target="_blank">View</a></div>
                {/if}
                <div class="field reason">{a.reason}</div>
              </div>
              <div class="card-actions">
                <button class="btn-action btn-approve" onclick={() => openResolve('appeal', a.id)}>Review</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

    <!-- Bans -->
    {:else if tab === 'bans'}
      <div class="section">
        <h3>Ban User</h3>
        <div class="inline-form">
          <input type="text" bind:value={banTarget} placeholder="User DID" />
          <input type="text" bind:value={banReason} placeholder="Reason (optional)" />
          <button class="btn-danger" onclick={doBan} disabled={!banTarget}>Ban</button>
        </div>
      </div>

      <h3>Banned Users ({bannedUsers.length})</h3>
      {#if bannedUsers.length === 0}
        <div class="empty">No banned users</div>
      {:else}
        <table class="data-table">
          <thead>
            <tr>
              <th>Handle</th>
              <th>Name</th>
              <th>Banned At</th>
              <th>Reason</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each bannedUsers as u}
              <tr>
                <td>@{u.handle}</td>
                <td>{u.display_name || '—'}</td>
                <td>{fmtDate(u.banned_at)}</td>
                <td>{u.ban_reason || '—'}</td>
                <td><button class="btn-action btn-small" onclick={() => doUnban(u.did)}>Unban</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}

    <!-- Users -->
    {:else if tab === 'users'}
      <div class="section">
        <h3>Delete Article</h3>
        <div class="inline-form">
          <input type="text" bind:value={deleteUri} placeholder="Article AT URI" />
          <input type="text" bind:value={deleteReason} placeholder="Reason" />
          <button class="btn-danger" onclick={doDelete} disabled={!deleteUri}>Delete</button>
        </div>
      </div>

      <h3>Platform Users ({users.length})</h3>
      <table class="data-table">
        <thead>
          <tr>
            <th>Handle</th>
            <th>Display Name</th>
            <th>Registered</th>
            <th>DID</th>
          </tr>
        </thead>
        <tbody>
          {#each users as u}
            <tr>
              <td>@{u.handle}</td>
              <td>{u.display_name || '—'}</td>
              <td>{fmtDate(u.created_at)}</td>
              <td class="mono">{u.did}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <!-- Resolve Modal -->
  {#if resolveTarget}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-overlay" onclick={() => resolveTarget = null}>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <h3>Resolve {resolveTarget.type === 'report' ? 'Report' : 'Appeal'}</h3>

        <div class="resolve-options">
          {#if resolveTarget.type === 'report'}
            <label><input type="radio" bind:group={resolveStatus} value="resolved" /> Resolved (action taken)</label>
            <label><input type="radio" bind:group={resolveStatus} value="dismissed" /> Dismissed (no action)</label>
          {:else}
            <label><input type="radio" bind:group={resolveStatus} value="approved" /> Approve (unban / restore)</label>
            <label><input type="radio" bind:group={resolveStatus} value="rejected" /> Reject</label>
          {/if}
        </div>

        <label>
          {resolveTarget.type === 'report' ? 'Admin note' : 'Response to user'}
          <textarea bind:value={resolveNote} rows="3" placeholder="Optional note..."></textarea>
        </label>

        <div class="modal-actions">
          <button class="btn-cancel" onclick={() => resolveTarget = null}>Cancel</button>
          <button class="btn-primary" onclick={submitResolve} disabled={!resolveStatus}>Submit</button>
        </div>
      </div>
    </div>
  {/if}
{/if}

<style>
  .auth-box {
    max-width: 360px;
    margin: 8vh auto;
    padding: 2rem;
    background: var(--bg-white);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .auth-box h1 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 0 0 1rem;
  }
  .auth-box label { display: block; font-size: 13px; font-weight: 500; color: var(--text-secondary); margin-bottom: 12px; }
  .auth-box input { display: block; width: 100%; margin-top: 4px; padding: 8px 10px; font-size: 14px; border: 1px solid var(--border); border-radius: 4px; font-family: var(--font-sans); background: var(--bg-white); }
  .error { background: #fef2f2; color: #dc2626; padding: 8px 12px; border-radius: 4px; font-size: 13px; margin-bottom: 12px; }

  .admin { max-width: 960px; margin: 0 auto; }
  .admin-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; }
  .admin-header h1 { font-family: var(--font-serif); font-weight: 400; margin: 0; }

  .admin-tabs { display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: 1.5rem; }
  .admin-tabs button { padding: 10px 16px; font-size: 14px; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-secondary); cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .admin-tabs button.active { color: var(--accent); border-bottom-color: var(--accent); }

  .badge, .badge-muted { font-size: 11px; font-weight: 600; padding: 1px 6px; border-radius: 10px; }
  .badge { background: #dc2626; color: white; }
  .badge-muted { background: var(--border); color: var(--text-secondary); }

  .tab-bar { display: flex; gap: 4px; margin-bottom: 1rem; }
  .tab-bar button { padding: 4px 12px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; }
  .tab-bar button.active { background: var(--accent); color: white; border-color: var(--accent); }

  .empty { padding: 2rem; text-align: center; color: var(--text-hint); font-size: 14px; }

  .card-list { display: flex; flex-direction: column; gap: 12px; }
  .card { border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
  .card.resolved { opacity: 0.6; }
  .card-header { display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: var(--bg-page); border-bottom: 1px solid var(--border); font-size: 13px; }
  .card-body { padding: 12px 14px; }
  .card-actions { display: flex; gap: 6px; padding: 8px 14px; border-top: 1px solid var(--border); background: var(--bg-page); }

  .kind-badge { font-size: 11px; font-weight: 600; text-transform: uppercase; padding: 2px 6px; border-radius: 3px; background: var(--border); color: var(--text-secondary); }
  .status-badge { font-size: 11px; font-weight: 600; padding: 2px 6px; border-radius: 3px; }
  .status-badge.pending { background: #fef3c7; color: #92400e; }
  .status-badge.dismissed { background: #f3f4f6; color: #6b7280; }

  .field { font-size: 13px; color: var(--text-primary); margin-bottom: 4px; }
  .field a { color: var(--accent); }
  .reason { white-space: pre-wrap; margin-top: 8px; padding: 8px; background: var(--bg-page); border-radius: 4px; border: 1px solid var(--border); }
  .admin-note { font-style: italic; color: var(--text-secondary); }
  .date { margin-left: auto; color: var(--text-hint); font-size: 12px; }

  .btn-primary { padding: 6px 14px; font-size: 13px; border: none; border-radius: 3px; background: var(--accent); color: white; cursor: pointer; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-refresh { padding: 4px 12px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; }
  .btn-cancel { padding: 6px 14px; font-size: 13px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; }

  .btn-action { padding: 4px 10px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: none; cursor: pointer; color: var(--text-secondary); }
  .btn-action:hover { border-color: var(--accent); color: var(--accent); }
  .btn-resolve { color: var(--accent); border-color: var(--accent); }
  .btn-approve { color: #059669; border-color: #059669; }
  .btn-dismiss { color: var(--text-hint); }
  .btn-danger, .btn-action.btn-danger { color: #dc2626; border-color: #dc2626; }
  .btn-danger:hover { background: #dc2626; color: white; }
  .btn-small { padding: 2px 8px; font-size: 11px; }

  .section { margin-bottom: 1.5rem; }
  .section h3 { font-size: 14px; font-weight: 600; margin: 0 0 8px; }
  h3 { font-size: 14px; font-weight: 600; margin: 1.5rem 0 8px; }

  .inline-form { display: flex; gap: 8px; align-items: center; }
  .inline-form input { padding: 6px 10px; font-size: 13px; border: 1px solid var(--border); border-radius: 3px; font-family: var(--font-sans); flex: 1; }

  .data-table { width: 100%; border-collapse: collapse; font-size: 13px; }
  .data-table th { text-align: left; padding: 8px 10px; border-bottom: 2px solid var(--border); color: var(--text-secondary); font-weight: 600; font-size: 12px; }
  .data-table td { padding: 6px 10px; border-bottom: 1px solid var(--border); }
  .mono { font-family: var(--font-mono, monospace); font-size: 11px; color: var(--text-hint); max-width: 200px; overflow: hidden; text-overflow: ellipsis; }

  .modal-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.3); z-index: 300; display: flex; justify-content: center; padding-top: 15vh; }
  .modal { width: 440px; max-width: 90vw; background: var(--bg-white); border-radius: 8px; padding: 24px; box-shadow: 0 8px 32px rgba(0,0,0,0.15); max-height: 70vh; overflow-y: auto; }
  .modal h3 { margin: 0 0 16px; font-family: var(--font-serif); font-weight: 400; }
  .modal label { display: block; font-size: 13px; font-weight: 500; color: var(--text-secondary); margin-bottom: 12px; }
  .modal textarea { display: block; width: 100%; margin-top: 4px; padding: 8px 10px; font-size: 13px; border: 1px solid var(--border); border-radius: 4px; font-family: var(--font-sans); resize: vertical; }
  .resolve-options { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }
  .resolve-options label { display: flex; align-items: center; gap: 8px; font-weight: 400; cursor: pointer; }
  .modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
</style>
