<script module lang="ts">
  import type { EditorView } from 'prosemirror-view';
  import type { EditorState } from 'prosemirror-state';

  export type ToolbarItem = {
    icon: string;                              // innerHTML for button (SVG or text)
    title: string;
    run: (view: EditorView) => void;
    active?: (state: EditorState) => boolean;
  };
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState, Plugin, TextSelection, type Transaction, type Command } from 'prosemirror-state';
  import { EditorView } from 'prosemirror-view';
  import { type Schema, type Node as PNode, type MarkType, type NodeType } from 'prosemirror-model';
  import { buildInputRules, buildKeymap } from 'prosemirror-example-setup';
  import { columnResizing, tableEditing, goToNextCell } from 'prosemirror-tables';
  import { keymap } from 'prosemirror-keymap';
  import { baseKeymap, toggleMark, setBlockType, wrapIn } from 'prosemirror-commands';
  import { history, undo, redo, undoDepth, redoDepth } from 'prosemirror-history';
  import { wrapInList } from 'prosemirror-schema-list';

  let {
    value = $bindable(''),
    placeholder = '',
    fillHeight = false,
    schema,
    plugins = [],
    nodeViews = {},
    serialize,
    parse,
    headingPrefixes = ['', '', ''] as [string, string, string],
    toolbarItems = [] as ToolbarItem[],
  }: {
    value: string;
    placeholder?: string;
    fillHeight?: boolean;
    schema: Schema;
    plugins?: Plugin[];
    nodeViews?: Record<string, (node: any, view: EditorView, getPos: any) => any>;
    serialize: (doc: PNode) => string;
    parse: (text: string) => PNode;
    headingPrefixes?: [string, string, string];
    toolbarItems?: ToolbarItem[];
  } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  let editorState = $state<EditorState | null>(null);
  let updating = false;
  let lastSyncedValue = '';
  let fullscreen = $state(false);

  const prefixChar = headingPrefixes[0]?.trimEnd()[0] ?? '';

  // ── State query helpers ───────────────────────────────────────────────────
  function isMarkActive(st: EditorState, markType: MarkType): boolean {
    const sel = st.selection;
    const anchor = sel.$from;
    if (sel.empty) return !!markType.isInSet(st.storedMarks || anchor.marks());
    return st.doc.rangeHasMark(sel.from, sel.to, markType);
  }

  function isBlockType(st: EditorState, nodeType: NodeType, attrs?: Record<string, any>): boolean {
    const sel = st.selection;
    const anchor = sel.$from;
    return sel.to <= anchor.end() && anchor.parent.hasMarkup(nodeType, attrs);
  }

  function runCmd(cmd: Command) {
    if (!view) return;
    cmd(view.state, view.dispatch, view);
    view.focus();
  }

  function insertTable() {
    if (!view) return;
    const { state, dispatch } = view;
    const { table, table_row, table_cell, table_header } = schema.nodes;
    if (!table) return;
    const headerCells = [0, 1, 2].map(() => table_header.createAndFill()!);
    const bodyCells   = [0, 1, 2].map(() => table_cell.createAndFill()!);
    dispatch(state.tr.replaceSelectionWith(
      table.create(null, [table_row.create(null, headerCells), table_row.create(null, bodyCells)])
    ));
    view.focus();
  }

  // ── Reactive toolbar state ────────────────────────────────────────────────
  let boldActive   = $derived(editorState && schema.marks?.strong   ? isMarkActive(editorState, schema.marks.strong)   : false);
  let italicActive = $derived(editorState && schema.marks?.em       ? isMarkActive(editorState, schema.marks.em)       : false);
  let codeActive   = $derived(editorState && schema.marks?.code     ? isMarkActive(editorState, schema.marks.code)     : false);
  let h1Active     = $derived(editorState && schema.nodes?.heading  ? isBlockType(editorState, schema.nodes.heading, { level: 1 }) : false);
  let h2Active     = $derived(editorState && schema.nodes?.heading  ? isBlockType(editorState, schema.nodes.heading, { level: 2 }) : false);
  let h3Active     = $derived(editorState && schema.nodes?.heading  ? isBlockType(editorState, schema.nodes.heading, { level: 3 }) : false);
  let canUndo      = $derived(editorState ? undoDepth(editorState) > 0 : false);
  let canRedo      = $derived(editorState ? redoDepth(editorState) > 0 : false);

  // ── Heading plugin: active prefix via CSS + level editing via keyboard ────
  const headingPlugin = new Plugin({
    props: {
      handleKeyDown(editorView, e) {
        const { state } = editorView;
        const sel = state.selection;
        if (!(sel instanceof TextSelection)) return false;
        const cursor = (sel as TextSelection).$cursor;
        if (!cursor || cursor.parent.type.name !== 'heading') return false;
        if (cursor.parentOffset !== 0) return false;

        const level = cursor.parent.attrs.level as number;
        const pos   = cursor.before();
        const end   = pos + cursor.parent.nodeSize;

        if (e.key === 'Backspace') {
          if (level <= 1) {
            editorView.dispatch(state.tr.setBlockType(pos, end, state.schema.nodes.paragraph));
          } else {
            editorView.dispatch(state.tr.setNodeMarkup(pos, null, { level: level - 1 }));
          }
          return true;
        }
        if (prefixChar && e.key === prefixChar && level < 6) {
          editorView.dispatch(state.tr.setNodeMarkup(pos, null, { level: level + 1 }));
          return true;
        }
        return false;
      },
    },
    view(editorView) {
      function update() {
        editorView.dom.querySelectorAll('[data-heading-active]').forEach(el => {
          (el as Element).removeAttribute('data-heading-active');
        });
        const { from } = editorView.state.selection;
        const pos = editorView.state.doc.resolve(from);
        for (let d = pos.depth; d >= 0; d--) {
          if (pos.node(d).type.name === 'heading') {
            const dom = editorView.nodeDOM(pos.before(d));
            if (dom instanceof Element) dom.setAttribute('data-heading-active', '');
            break;
          }
        }
      }
      update();
      return { update };
    },
  });

  onMount(() => {
    const allPlugins = [
      ...plugins,
      buildInputRules(schema),
      keymap(buildKeymap(schema)),
      keymap(baseKeymap),
      history(),
      columnResizing(),
      tableEditing(),
      keymap({ 'Tab': goToNextCell(1), 'Shift-Tab': goToNextCell(-1) }),
      headingPlugin,
    ];

    const initialState = EditorState.create({ doc: parse(value), plugins: allPlugins });
    lastSyncedValue = value;

    view = new EditorView(container, {
      state: initialState,
      nodeViews,
      dispatchTransaction(tr: Transaction) {
        if (!view) return;
        const newState = view.state.apply(tr);
        view.updateState(newState);
        if (!updating && tr.docChanged) {
          updating = true;
          try { value = serialize(newState.doc); lastSyncedValue = value; } catch {}
          updating = false;
        }
        // Defer toolbar state update: avoid Svelte DOM flush conflicting with
        // ProseMirror's own DOM reconciliation in the same synchronous frame.
        queueMicrotask(() => { if (view) editorState = view.state; });
      },
    });
    queueMicrotask(() => { editorState = initialState; });

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && fullscreen) fullscreen = false;
    };
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('keydown', handleKey);
      view?.destroy();
      view = null;
    };
  });

  // Only sync when value changes from outside (not from dispatchTransaction)
  $effect(() => {
    if (!view || updating || value === lastSyncedValue) return;
    // Normalize both sides to avoid infinite loops from serialization differences
    let normalized: string;
    try { normalized = serialize(parse(value)); } catch { normalized = value; }
    const current = serialize(view.state.doc);
    if (normalized !== current) {
      updating = true;
      const newDoc = parse(value);
      const tr = view.state.tr.replaceWith(0, view.state.doc.content.size, newDoc.content);
      view.dispatch(tr);
      updating = false;
    }
    lastSyncedValue = value;
  });
</script>

<div
  class="md-editor-wrapper"
  class:fill-height={fillHeight}
  class:fullscreen
  style="--h1-prefix: '{headingPrefixes[0]}'; --h2-prefix: '{headingPrefixes[1]}'; --h3-prefix: '{headingPrefixes[2]}';"
>
  <!-- ── Toolbar ── -->
  <div class="prose-toolbar" class:fill-height={fillHeight}>
    <button class="tb-btn" onmousedown={(e) => { e.preventDefault(); runCmd(undo); }} title="撤销 (Ctrl+Z)" disabled={!canUndo}>
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M1.5 6.5a5 5 0 1 0 1.1-3"/><polyline points="1.5,1.5 1.5,5.5 5.5,5.5"/></svg>
    </button>
    <button class="tb-btn" onmousedown={(e) => { e.preventDefault(); runCmd(redo); }} title="重做 (Ctrl+Y)" disabled={!canRedo}>
      <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M11.5 6.5a5 5 0 1 1-1.1-3"/><polyline points="11.5,1.5 11.5,5.5 7.5,5.5"/></svg>
    </button>

    <span class="tb-sep"></span>

    {#if schema.marks?.strong}
      <button class="tb-btn" class:active={boldActive}
        onmousedown={(e) => { e.preventDefault(); runCmd(toggleMark(schema.marks.strong)); }}
        title="加粗 (Ctrl+B)"><b>B</b></button>
    {/if}
    {#if schema.marks?.em}
      <button class="tb-btn" class:active={italicActive}
        onmousedown={(e) => { e.preventDefault(); runCmd(toggleMark(schema.marks.em)); }}
        title="斜体 (Ctrl+I)"><i>I</i></button>
    {/if}
    {#if schema.marks?.code}
      <button class="tb-btn" class:active={codeActive}
        onmousedown={(e) => { e.preventDefault(); runCmd(toggleMark(schema.marks.code)); }}
        title="行内代码"><code>`</code></button>
    {/if}

    {#if schema.nodes?.heading}
      <span class="tb-sep"></span>
      <button class="tb-btn" class:active={h1Active}
        onmousedown={(e) => { e.preventDefault(); runCmd(setBlockType(schema.nodes.heading, { level: 1 })); }}
        title="一级标题">H1</button>
      <button class="tb-btn" class:active={h2Active}
        onmousedown={(e) => { e.preventDefault(); runCmd(setBlockType(schema.nodes.heading, { level: 2 })); }}
        title="二级标题">H2</button>
      <button class="tb-btn" class:active={h3Active}
        onmousedown={(e) => { e.preventDefault(); runCmd(setBlockType(schema.nodes.heading, { level: 3 })); }}
        title="三级标题">H3</button>
    {/if}

    <span class="tb-sep"></span>

    {#if schema.nodes?.bullet_list}
      <button class="tb-btn"
        onmousedown={(e) => { e.preventDefault(); runCmd(wrapInList(schema.nodes.bullet_list)); }}
        title="无序列表">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
          <circle cx="2" cy="4" r="1.1"/><circle cx="2" cy="10" r="1.1"/>
          <rect x="5" y="3.3" width="8" height="1.4" rx="0.6"/>
          <rect x="5" y="9.3" width="8" height="1.4" rx="0.6"/>
        </svg>
      </button>
    {/if}
    {#if schema.nodes?.ordered_list}
      <button class="tb-btn"
        onmousedown={(e) => { e.preventDefault(); runCmd(wrapInList(schema.nodes.ordered_list)); }}
        title="有序列表">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor" font-size="5" font-family="monospace">
          <text x="1" y="5.5">1.</text><text x="1" y="11">2.</text>
          <rect x="6.5" y="3.3" width="6.5" height="1.4" rx="0.6"/>
          <rect x="6.5" y="9.3" width="6.5" height="1.4" rx="0.6"/>
        </svg>
      </button>
    {/if}
    {#if schema.nodes?.blockquote}
      <button class="tb-btn"
        onmousedown={(e) => { e.preventDefault(); runCmd(wrapIn(schema.nodes.blockquote)); }}
        title="引用">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="2" width="2" height="10" rx="1" fill="currentColor" stroke="none"/>
          <line x1="6" y1="5" x2="12" y2="5"/><line x1="6" y1="9" x2="12" y2="9"/>
        </svg>
      </button>
    {/if}
    {#if schema.nodes?.code_block}
      <button class="tb-btn"
        onmousedown={(e) => { e.preventDefault(); runCmd(setBlockType(schema.nodes.code_block)); }}
        title="代码块">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
          <polyline points="4.5,4 1.5,7 4.5,10"/><polyline points="9.5,4 12.5,7 9.5,10"/>
          <line x1="8.5" y1="2" x2="5.5" y2="12"/>
        </svg>
      </button>
    {/if}
    {#if schema.nodes?.table}
      <button class="tb-btn" onmousedown={(e) => { e.preventDefault(); insertTable(); }} title="插入表格">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4">
          <rect x="1" y="2" width="12" height="10" rx="1"/>
          <line x1="1" y1="5.5" x2="13" y2="5.5"/>
          <line x1="1" y1="9" x2="13" y2="9"/>
          <line x1="5.5" y1="2" x2="5.5" y2="12"/>
          <line x1="9" y1="2" x2="9" y2="12"/>
        </svg>
      </button>
    {/if}

    {#if toolbarItems.length > 0}
      <span class="tb-sep"></span>
      {#each toolbarItems as item}
        <button
          class="tb-btn"
          class:active={editorState && item.active ? item.active(editorState) : false}
          onmousedown={(e) => { e.preventDefault(); if (view) item.run(view); view?.focus(); }}
          title={item.title}
        >{@html item.icon}</button>
      {/each}
    {/if}

    <span class="tb-spacer"></span>
    <button class="tb-btn" class:active={fullscreen} onmousedown={(e) => { e.preventDefault(); fullscreen = !fullscreen; }} title="全屏">
      {#if fullscreen}
        <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4.5 1.5H1.5v3M8.5 1.5h3v3M8.5 11.5h3v-3M4.5 11.5h-3v-3"/></svg>
      {:else}
        <svg width="13" height="13" viewBox="0 0 13 13" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M1.5 4.5v-3h3M8.5 1.5h3v3M11.5 8.5v3h-3M4.5 11.5h-3v-3"/></svg>
      {/if}
    </button>
  </div>

  <div class="md-editor" bind:this={container}></div>
  {#if !value && placeholder}
    <div class="md-placeholder">{placeholder}</div>
  {/if}
</div>

<style>
  .md-editor-wrapper {
    position: relative;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    display: flex;
    flex-direction: column;
  }
  .md-editor-wrapper.fill-height {
    flex: 1;
    min-height: 0;
    border: none;
    border-radius: 0;
    background: var(--bg-page);
  }
  .md-editor-wrapper.fullscreen {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    z-index: 9999;
    border-radius: 0;
    border: none;
  }

  /* ── Toolbar ── */
  .prose-toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 1px;
    padding: 3px 6px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
    min-height: 32px;
    flex-shrink: 0;
  }
  .prose-toolbar.fill-height {
    background: var(--bg-page);
    border-bottom-color: transparent;
    padding-left: max(1rem, calc(50% - 364px));
    padding-right: max(1rem, calc(50% - 364px));
  }
  .tb-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 5px;
    min-width: 22px;
    height: 22px;
    border: none;
    background: none;
    border-radius: 3px;
    cursor: pointer;
    color: var(--text-primary);
    font-size: 12px;
    line-height: 1;
    transition: background 0.1s;
  }
  .tb-btn:hover:not(:disabled) { background: var(--bg-hover); }
  .tb-btn.active { background: var(--accent); color: white; }
  .tb-btn:disabled { opacity: 0.35; cursor: default; }
  .tb-btn b, .tb-btn i, .tb-btn code {
    font-size: 13px;
    font-style: normal;
    font-family: var(--font-sans, sans-serif);
  }
  .tb-btn b { font-weight: 700; }
  .tb-btn i { font-style: italic; }
  .tb-btn code { font-family: var(--font-mono, monospace); font-size: 13px; }
  .tb-sep { width: 1px; height: 16px; background: var(--border); margin: 0 3px; flex-shrink: 0; }
  .tb-spacer { flex: 1; }

  /* ── Editor area ── */
  .md-editor {
    flex: 1;
    min-height: 300px;
    overflow-y: auto;
    position: relative;
  }
  .fill-height .md-editor,
  .fullscreen .md-editor {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .md-placeholder {
    position: absolute;
    color: var(--text-hint);
    font-size: 12pt;
    font-family: var(--font-serif);
    pointer-events: none;
    padding: 1.5rem 14px;
    top: 40px;
  }
  .fill-height .md-placeholder {
    left: max(1rem, calc(50% - 364px));
    padding-left: 1rem;
    padding-right: 1rem;
  }

  /* ── ProseMirror content ── */
  .md-editor :global(.ProseMirror) {
    padding: 1.5rem 14px;
    min-height: 280px;
    outline: none;
    white-space: pre-wrap;
    word-wrap: break-word;
    font-family: var(--font-serif);
    font-size: 12pt;
    line-height: 1.45;
    color: var(--text-primary);
    text-align: left;
    hyphens: auto;
  }
  .fill-height .md-editor :global(.ProseMirror) {
    flex: 1;
    min-height: 0;
    max-width: 760px;
    margin: 0 auto;
    padding: 0.75rem 1rem 2rem;
    width: 100%;
    box-sizing: border-box;
  }
  .fullscreen .md-editor :global(.ProseMirror) {
    flex: 1;
    min-height: 0;
    max-width: 760px;
    margin: 0 auto;
    padding: 2rem 1rem;
    width: 100%;
    box-sizing: border-box;
  }

  .md-editor :global(.ProseMirror p)  { margin: 1em 0; overflow-wrap: break-word; }

  .md-editor :global(.ProseMirror h1) { font-family: var(--font-serif); font-size: 2rem;   font-weight: 400; margin: 2em 0 0.5em; }
  .md-editor :global(.ProseMirror h2) { font-family: var(--font-serif); font-size: 1.6rem; font-weight: 400; margin: 1.75em 0 0.5em; padding-bottom: 0.25em; border-bottom: 1px solid var(--border); }
  .md-editor :global(.ProseMirror h3) { font-family: var(--font-serif); font-size: 1.2rem; font-weight: 600; margin: 1.5em 0 0.4em; }

  /* Heading prefix — only when cursor is inside the heading */
  .md-editor :global(.ProseMirror h1[data-heading-active])::before { content: var(--h1-prefix, ''); font-family: var(--font-mono, monospace); font-size: 0.55em; font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }
  .md-editor :global(.ProseMirror h2[data-heading-active])::before { content: var(--h2-prefix, ''); font-family: var(--font-mono, monospace); font-size: 0.6em;  font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }
  .md-editor :global(.ProseMirror h3[data-heading-active])::before { content: var(--h3-prefix, ''); font-family: var(--font-mono, monospace); font-size: 0.7em;  font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }

  .md-editor :global(.ProseMirror code)      { font-size: 0.9em; padding: 0.15em 0.35em; background: var(--bg-gray, #f5f5f5); border-radius: 3px; }
  .md-editor :global(.ProseMirror pre)       { overflow-x: auto; padding: 1em; margin: 1em 0; background: var(--bg-gray, #f5f5f5); border-radius: 4px; font-size: 0.9em; line-height: 1.5; }
  .md-editor :global(.ProseMirror pre code)  { padding: 0; background: none; }
  .md-editor :global(.ProseMirror blockquote){ margin: 1em 0; padding: 0.5em 1em; border-left: 3px solid var(--border-strong); color: var(--text-secondary); }
  .md-editor :global(.ProseMirror ul),
  .md-editor :global(.ProseMirror ol)        { padding-left: 1.5em; margin: 0.75em 0; }
  .md-editor :global(.ProseMirror li)        { margin: 0.25em 0; }
  .md-editor :global(.ProseMirror hr)        { border: none; border-top: 1px solid var(--border); margin: 1em 0; }
  .md-editor :global(.ProseMirror img)       { max-width: 100%; height: auto; }
  .md-editor :global(.ProseMirror-focused)   { outline: none; }
  .md-editor :global(.ProseMirror ::selection) { background: rgba(95, 155, 101, 0.25); }

  /* Gap cursor (prosemirror-gapcursor not installed, but defensive) */
  .md-editor :global(.ProseMirror-gapcursor) { display: none; pointer-events: none; position: absolute; }

  /* ── Tables ── */
  .md-editor :global(table)              { border-collapse: collapse; margin: 1.25em auto; font-size: 0.95em; width: auto; overflow: auto; }
  .md-editor :global(th),
  .md-editor :global(td)                { border: 1px solid var(--border-strong); padding: 0.5em 0.875em; min-width: 60px; text-align: left; vertical-align: top; position: relative; }
  .md-editor :global(th)                { font-weight: 600; font-family: var(--font-sans); font-size: 0.85em; text-transform: uppercase; letter-spacing: 0.03em; }
  .md-editor :global(.selectedCell)     { background: rgba(95, 155, 101, 0.15); }
  .md-editor :global(.column-resize-handle) { position: absolute; right: -2px; top: 0; bottom: 0; width: 4px; background: var(--accent, #4a7); cursor: col-resize; z-index: 20; }
</style>
