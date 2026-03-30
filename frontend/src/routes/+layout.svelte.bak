<script lang="ts">
  import '../app.css';
  import { QueryClient } from '@tanstack/svelte-query';
  import { QueryClientProvider } from '@tanstack/svelte-query';
  import Toast from '$lib/components/Toast.svelte';
  import KeyboardShortcuts from '$lib/components/KeyboardShortcuts.svelte';
  import { goto } from '$app/navigation';

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        gcTime: 5 * 60_000,
        refetchOnWindowFocus: false,
        retry: 1,
      },
    },
  });

  let { children } = $props();

  // Redirect old hash URLs to clean paths
  $effect(() => {
    if (window.location.hash.startsWith('#/')) {
      const path = window.location.hash.slice(1);
      goto(path, { replaceState: true });
    }
  });
</script>

<QueryClientProvider client={queryClient}>
  <Toast />
  <KeyboardShortcuts />
  {@render children()}
</QueryClientProvider>
