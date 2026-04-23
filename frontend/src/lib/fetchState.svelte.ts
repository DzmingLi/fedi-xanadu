// Reactive fetch-state holder for routes and components.
//
// Consolidates the { data, error, loading } triple + try/catch/abort boilerplate
// that was inlined in ~40 route files. Opt-in: existing code keeps working,
// new or refactored routes can use `createFetchState()` instead of hand-rolling
// reactive state variables.
//
// Usage:
//   const articles = createFetchState<Article[]>();
//   $effect(() => {
//     const ac = new AbortController();
//     articles.run(() => listArticles(ac.signal), ac.signal);
//     return () => ac.abort();
//   });
//   {#if articles.loading}…{:else if articles.error}…{:else if articles.data}…

export interface FetchState<T> {
  readonly data: T | null;
  readonly error: Error | null;
  readonly loading: boolean;
  run(fn: () => Promise<T>, signal?: AbortSignal): Promise<void>;
  set(value: T | null): void;
  reset(): void;
}

export function createFetchState<T>(initial: T | null = null): FetchState<T> {
  let data = $state<T | null>(initial);
  let error = $state<Error | null>(null);
  let loading = $state(false);

  async function run(fn: () => Promise<T>, signal?: AbortSignal): Promise<void> {
    loading = true;
    error = null;
    try {
      const result = await fn();
      if (!signal?.aborted) data = result;
    } catch (e: unknown) {
      if (signal?.aborted) return;
      error = e instanceof Error ? e : new Error(String(e));
    } finally {
      if (!signal?.aborted) loading = false;
    }
  }

  return {
    get data() { return data; },
    get error() { return error; },
    get loading() { return loading; },
    run,
    set(value: T | null) { data = value; },
    reset() { data = initial; error = null; loading = false; },
  };
}
