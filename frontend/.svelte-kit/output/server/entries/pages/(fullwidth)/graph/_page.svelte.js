import { e as ensure_array_like, c as escape_html, i as attr, b as stringify, a as attr_class, j as derived, k as store_get, u as unsubscribe_stores } from "../../../../chunks/index2.js";
import "../../../../chunks/auth.svelte.js";
import { c as createQuery } from "../../../../chunks/createQuery.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    let graphQuery = createQuery({});
    let treeQuery = createQuery({});
    let loading = derived(() => store_get($$store_subs ??= {}, "$graphQuery", graphQuery).isPending || store_get($$store_subs ??= {}, "$treeQuery", treeQuery).isPending);
    let nodes = derived(() => store_get($$store_subs ??= {}, "$graphQuery", graphQuery).data?.nodes ?? []);
    let edges = derived(() => store_get($$store_subs ??= {}, "$graphQuery", graphQuery).data?.edges ?? []);
    let tree = derived(() => store_get($$store_subs ??= {}, "$treeQuery", treeQuery).data ?? []);
    let categories = derived(() => {
      if (loading()) return [];
      const nodeMap = new Map(nodes().map((n) => [n.id, n]));
      const childrenOf = /* @__PURE__ */ new Map();
      const hasParent = /* @__PURE__ */ new Set();
      for (const e of tree()) {
        const arr = childrenOf.get(e.parent_tag) || [];
        arr.push(e.child_tag);
        childrenOf.set(e.parent_tag, arr);
        hasParent.add(e.child_tag);
      }
      const roots = [];
      for (const [parent] of childrenOf) {
        if (!hasParent.has(parent)) {
          roots.push(parent);
        }
      }
      function buildTree(id) {
        const node = nodeMap.get(id);
        if (!node) return null;
        const childIds = childrenOf.get(id) || [];
        const children = childIds.map((c) => buildTree(c)).filter((c) => c !== null);
        return { id: node.id, name: node.name, lit: node.lit, children };
      }
      const cats = roots.map((r) => buildTree(r)).filter((c) => c !== null);
      const inTree = /* @__PURE__ */ new Set();
      function collectIds(t) {
        inTree.add(t.id);
        t.children.forEach(collectIds);
      }
      cats.forEach(collectIds);
      const orphans = nodes().filter((n) => !inTree.has(n.id)).map((n) => ({ id: n.id, name: n.name, lit: n.lit, children: [] }));
      if (orphans.length > 0) {
        cats.push({ id: "__other", name: "Other", lit: false, children: orphans });
      }
      return cats;
    });
    let prereqInfo = derived(() => {
      const nodeMap = new Map(nodes().map((n) => [n.id, n]));
      return edges().map((e) => ({
        from: nodeMap.get(e.from)?.name || e.from,
        to: nodeMap.get(e.to)?.name || e.to,
        type: e.type
      }));
    });
    $$renderer2.push(`<div class="graph-page svelte-1b0lefm"><h1>Knowledge Map</h1> `);
    if (loading()) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<p class="meta">Loading...</p>`);
    } else {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="categories svelte-1b0lefm"><!--[-->`);
      const each_array = ensure_array_like(categories());
      for (let $$index_2 = 0, $$length = each_array.length; $$index_2 < $$length; $$index_2++) {
        let cat = each_array[$$index_2];
        $$renderer2.push(`<div class="category svelte-1b0lefm"><div class="category-header svelte-1b0lefm"><span class="category-name svelte-1b0lefm">${escape_html(cat.name)}</span></div> <div class="category-body svelte-1b0lefm">`);
        if (cat.children.length === 0) {
          $$renderer2.push("<!--[0-->");
          $$renderer2.push(`<a${attr("href", `/tag?id=${stringify(encodeURIComponent(cat.id))}`)}${attr_class("tag-node svelte-1b0lefm", void 0, { "lit": cat.lit })}>${escape_html(cat.name)}</a>`);
        } else {
          $$renderer2.push("<!--[-1-->");
          $$renderer2.push(`<!--[-->`);
          const each_array_1 = ensure_array_like(cat.children);
          for (let $$index_1 = 0, $$length2 = each_array_1.length; $$index_1 < $$length2; $$index_1++) {
            let sub = each_array_1[$$index_1];
            if (sub.children.length > 0) {
              $$renderer2.push("<!--[0-->");
              $$renderer2.push(`<div class="subcategory svelte-1b0lefm"><a${attr("href", `/tag?id=${stringify(encodeURIComponent(sub.id))}`)}${attr_class("sub-header svelte-1b0lefm", void 0, { "lit": sub.lit })}>${escape_html(sub.name)}</a> <div class="sub-tags svelte-1b0lefm"><!--[-->`);
              const each_array_2 = ensure_array_like(sub.children);
              for (let $$index = 0, $$length3 = each_array_2.length; $$index < $$length3; $$index++) {
                let leaf = each_array_2[$$index];
                $$renderer2.push(`<a${attr("href", `/tag?id=${stringify(encodeURIComponent(leaf.id))}`)}${attr_class("tag-node svelte-1b0lefm", void 0, { "lit": leaf.lit })}>${escape_html(leaf.name)}</a>`);
              }
              $$renderer2.push(`<!--]--></div></div>`);
            } else {
              $$renderer2.push("<!--[-1-->");
              $$renderer2.push(`<a${attr("href", `/tag?id=${stringify(encodeURIComponent(sub.id))}`)}${attr_class("tag-node svelte-1b0lefm", void 0, { "lit": sub.lit })}>${escape_html(sub.name)}</a>`);
            }
            $$renderer2.push(`<!--]-->`);
          }
          $$renderer2.push(`<!--]-->`);
        }
        $$renderer2.push(`<!--]--></div></div>`);
      }
      $$renderer2.push(`<!--]--></div> `);
      if (prereqInfo().length > 0) {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<div class="prereq-section svelte-1b0lefm"><h2 class="svelte-1b0lefm">Prerequisite Relationships</h2> <div class="prereq-list svelte-1b0lefm"><!--[-->`);
        const each_array_3 = ensure_array_like(prereqInfo());
        for (let $$index_3 = 0, $$length = each_array_3.length; $$index_3 < $$length; $$index_3++) {
          let p = each_array_3[$$index_3];
          $$renderer2.push(`<div class="prereq-edge svelte-1b0lefm"><span class="prereq-from">${escape_html(p.from)}</span> <span${attr_class(`prereq-arrow ${stringify(p.type)}`, "svelte-1b0lefm")}>→</span> <span class="prereq-to">${escape_html(p.to)}</span> <span${attr_class(`prereq-type ${stringify(p.type)}`, "svelte-1b0lefm")}>${escape_html(p.type)}</span></div>`);
        }
        $$renderer2.push(`<!--]--></div></div>`);
      } else {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <div class="legend svelte-1b0lefm"><span class="tag-node lit svelte-1b0lefm" style="font-size:12px;padding:2px 8px">mastered</span> <span class="tag-node svelte-1b0lefm" style="font-size:12px;padding:2px 8px">unlearned</span> <span class="prereq-type required svelte-1b0lefm">required</span> <span class="prereq-type recommended svelte-1b0lefm">recommended</span> <span class="prereq-type suggested svelte-1b0lefm">suggested</span></div>`);
    }
    $$renderer2.push(`<!--]--></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
