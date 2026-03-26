<script lang="ts" module>
  type ToastType = 'info' | 'success' | 'error';
  interface ToastItem { id: number; message: string; type: ToastType; }

  let toasts = $state<ToastItem[]>([]);
  let nextId = 0;

  export function toast(message: string, type: ToastType = 'info', duration = 3000) {
    const id = nextId++;
    toasts = [...toasts, { id, message, type }];
    setTimeout(() => {
      toasts = toasts.filter(t => t.id !== id);
    }, duration);
  }
</script>

{#if toasts.length > 0}
  <div class="toast-container">
    {#each toasts as t (t.id)}
      <div class="toast toast-{t.type}">
        {t.message}
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    top: 16px;
    right: 16px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    padding: 10px 20px;
    border-radius: 4px;
    font-size: 14px;
    font-family: var(--font-sans);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    pointer-events: auto;
    animation: toast-in 0.2s ease-out;
  }
  .toast-info {
    background: var(--text-primary, #333);
    color: #fff;
  }
  .toast-success {
    background: var(--accent, #5f9b65);
    color: #fff;
  }
  .toast-error {
    background: #dc2626;
    color: #fff;
  }
  @keyframes toast-in {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: translateY(0); }
  }
</style>
