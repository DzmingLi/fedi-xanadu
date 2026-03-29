import { QueryObserver } from "@tanstack/query-core";
import { j as derived } from "./index2.js";
import { g as getIsRestoringContext, a as getQueryClientContext } from "./context.js";
import "clsx";
function useIsRestoring() {
  return getIsRestoringContext();
}
function useQueryClient(queryClient) {
  return getQueryClientContext();
}
const SvelteSet = globalThis.Set;
(function(receiver, state, value, kind, f) {
  if (kind === "m") throw new TypeError("Private method is not writable");
  if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
  if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value), value;
});
(function(receiver, state, kind, f) {
  if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
  if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
});
function createRawRef(init) {
  const refObj = Array.isArray(init) ? [] : {};
  const hiddenKeys = new SvelteSet();
  const out = new Proxy(refObj, {
    set(target, prop, value, receiver) {
      hiddenKeys.delete(prop);
      if (prop in target) {
        return Reflect.set(target, prop, value, receiver);
      }
      let state = value;
      Object.defineProperty(target, prop, {
        configurable: true,
        enumerable: true,
        get: () => {
          return state && isBranded(state) ? state() : state;
        },
        set: (v) => {
          state = v;
        }
      });
      return true;
    },
    has: (target, prop) => {
      if (hiddenKeys.has(prop)) {
        return false;
      }
      return prop in target;
    },
    ownKeys(target) {
      return Reflect.ownKeys(target).filter((key) => !hiddenKeys.has(key));
    },
    getOwnPropertyDescriptor(target, prop) {
      if (hiddenKeys.has(prop)) {
        return void 0;
      }
      return Reflect.getOwnPropertyDescriptor(target, prop);
    },
    deleteProperty(target, prop) {
      if (prop in target) {
        target[prop] = void 0;
        hiddenKeys.add(prop);
        if (Array.isArray(target)) {
          target.length--;
        }
        return true;
      }
      return false;
    }
  });
  function update(newValue) {
    const existingKeys = Object.keys(out);
    const newKeys = Object.keys(newValue);
    const keysToRemove = existingKeys.filter((key) => !newKeys.includes(key));
    for (const key of keysToRemove) {
      delete out[key];
    }
    for (const key of newKeys) {
      out[key] = brand(() => newValue[key]);
    }
  }
  update(init);
  return [out, update];
}
const lazyBrand = Symbol("LazyValue");
function brand(fn) {
  fn[lazyBrand] = true;
  return fn;
}
function isBranded(fn) {
  return Boolean(fn[lazyBrand]);
}
function createBaseQuery(options, Observer, queryClient) {
  const client = derived(() => useQueryClient());
  const isRestoring = useIsRestoring();
  const resolvedOptions = derived(() => {
    const opts = client().defaultQueryOptions(options());
    opts._optimisticResults = isRestoring.current ? "isRestoring" : "optimistic";
    return opts;
  });
  let observer = new Observer(client(), resolvedOptions());
  function createResult() {
    const result = observer.getOptimisticResult(resolvedOptions());
    return !resolvedOptions().notifyOnChangeProps ? observer.trackResult(result) : result;
  }
  const [query] = createRawRef(
    // svelte-ignore state_referenced_locally - intentional, initial value
    createResult()
  );
  return query;
}
function createQuery(options, queryClient) {
  return createBaseQuery(options, QueryObserver);
}
export {
  createQuery as c
};
