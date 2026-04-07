<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState, Plugin, type Transaction } from 'prosemirror-state';
  import { EditorView } from 'prosemirror-view';
  import { type Schema, type Node as PNode } from 'prosemirror-model';
  import { exampleSetup } from 'prosemirror-example-setup';
  import { columnResizing, tableEditing, goToNextCell } from 'prosemirror-tables';
  import { keymap } from 'prosemirror-keymap';
  import { t } from '../i18n/index.svelte';

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
  } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  let updating = false;
  let fullscreen = $state(false);

  // Plugin: highlight the heading the cursor is currently inside
  const headingFocusPlugin = new Plugin({
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

  function insertTable() {
    if (!view) return;
    const { state, dispatch } = view;
    const { table, table_row, table_cell, table_header } = schema.nodes;
    const headerCells = [0, 1, 2].map(() => table_header.createAndFill()!);
    const bodyCells   = [0, 1, 2].map(() => table_cell.createAndFill()!);
    dispatch(state.tr.replaceSelectionWith(
      table.create(null, [table_row.create(null, headerCells), table_row.create(null, bodyCells)])
    ));
    view.focus();
  }

  onMount(() => {
    const editorState = EditorState.create({
      doc: parse(value),
      plugins: [
        ...plugins,
        ...exampleSetup({ schema }),
        columnResizing(),
        tableEditing(),
        keymap({ 'Tab': goToNextCell(1), 'Shift-Tab': goToNextCell(-1) }),
        headingFocusPlugin,
      ],
    });

    view = new EditorView(container, {
      state: editorState,
      nodeViews,
      dispatchTransaction(tr: Transaction) {
        if (!view) return;
        const newState = view.state.apply(tr);
        view.updateState(newState);
        if (!updating && tr.docChanged) {
          updating = true;
          try { value = serialize(newState.doc); } catch {}
          updating = false;
        }
      },
    });

    // Inject table button into ProseMirror menubar
    const menubar = container.querySelector('.ProseMirror-menubar');
    if (menubar) {
      const sep = document.createElement('span');
      sep.className = 'ProseMirror-menuseparator';
      menubar.appendChild(sep);
      const btn = document.createElement('span');
      btn.className = 'ProseMirror-icon table-menu-btn';
      btn.title = t('editor.insertTable');
      btn.innerHTML = `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="1" y="2" width="14" height="12" rx="1"/><line x1="1" y1="6" x2="15" y2="6"/><line x1="1" y1="10" x2="15" y2="10"/><line x1="6" y1="2" x2="6" y2="14"/><line x1="11" y1="2" x2="11" y2="14"/></svg>`;
      btn.addEventListener('mousedown', (e) => { e.preventDefault(); insertTable(); });
      menubar.appendChild(btn);
    }

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

  $effect(() => {
    if (!view || updating) return;
    const current = serialize(view.state.doc);
    if (value !== current) {
      updating = true;
      const newDoc = parse(value);
      const tr = view.state.tr.replaceWith(0, view.state.doc.content.size, newDoc.content);
      view.dispatch(tr);
      updating = false;
    }
  });
</script>

<div
  class="md-editor-wrapper"
  class:fill-height={fillHeight}
  class:fullscreen
  style="--h1-prefix: '{headingPrefixes[0]}'; --h2-prefix: '{headingPrefixes[1]}'; --h3-prefix: '{headingPrefixes[2]}';"
>
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

  /* ── ProseMirror editing area ── */
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

  .md-editor :global(.ProseMirror p) { margin: 1em 0; overflow-wrap: break-word; }

  .md-editor :global(.ProseMirror h1) { font-family: var(--font-serif); font-size: 2rem; font-weight: 400; margin: 2em 0 0.5em; }
  .md-editor :global(.ProseMirror h2) { font-family: var(--font-serif); font-size: 1.6rem; font-weight: 400; margin: 1.75em 0 0.5em; padding-bottom: 0.25em; border-bottom: 1px solid var(--border); }
  .md-editor :global(.ProseMirror h3) { font-family: var(--font-serif); font-size: 1.2rem; font-weight: 600; margin: 1.5em 0 0.4em; }

  /* Heading level prefix — only visible when cursor is inside (data-heading-active) */
  .md-editor :global(.ProseMirror h1[data-heading-active])::before { content: var(--h1-prefix, ''); font-family: var(--font-mono, monospace); font-size: 0.55em; font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }
  .md-editor :global(.ProseMirror h2[data-heading-active])::before { content: var(--h2-prefix, ''); font-family: var(--font-mono, monospace); font-size: 0.6em;  font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }
  .md-editor :global(.ProseMirror h3[data-heading-active])::before { content: var(--h3-prefix, ''); font-family: var(--font-mono, monospace); font-size: 0.7em;  font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }

  .md-editor :global(.ProseMirror code) { font-size: 0.9em; padding: 0.15em 0.35em; background: var(--bg-gray, #f5f5f5); border-radius: 3px; }
  .md-editor :global(.ProseMirror pre) { overflow-x: auto; padding: 1em; margin: 1em 0; background: var(--bg-gray, #f5f5f5); border-radius: 4px; font-size: 0.9em; line-height: 1.5; }
  .md-editor :global(.ProseMirror pre code) { padding: 0; background: none; }
  .md-editor :global(.ProseMirror blockquote) { margin: 1em 0; padding: 0.5em 1em; border-left: 3px solid var(--border-strong); color: var(--text-secondary); }
  .md-editor :global(.ProseMirror ul),
  .md-editor :global(.ProseMirror ol) { padding-left: 1.5em; margin: 0.75em 0; }
  .md-editor :global(.ProseMirror li) { margin: 0.25em 0; }
  .md-editor :global(.ProseMirror hr) { border: none; border-top: 1px solid var(--border); margin: 1em 0; }
  .md-editor :global(.ProseMirror img) { max-width: 100%; height: auto; }
  .md-editor :global(.ProseMirror-focused) { outline: none; }

  /* ── Tables ── */
  .md-editor :global(table) { border-collapse: collapse; margin: 1.25em auto; font-size: 0.95em; width: auto; overflow: auto; }
  .md-editor :global(th),
  .md-editor :global(td) { border: 1px solid var(--border-strong); padding: 0.5em 0.875em; min-width: 60px; text-align: left; vertical-align: top; position: relative; }
  .md-editor :global(th) { font-weight: 600; font-family: var(--font-sans); font-size: 0.85em; text-transform: uppercase; letter-spacing: 0.03em; }
  .md-editor :global(.selectedCell) { background: rgba(95, 155, 101, 0.15); }
  .md-editor :global(.column-resize-handle) { position: absolute; right: -2px; top: 0; bottom: 0; width: 4px; background: var(--accent, #4a7); cursor: col-resize; z-index: 20; }

  /* ── Menu bar ── */
  .md-editor :global(.ProseMirror-menubar-wrapper) { border: none; display: flex; flex-direction: column; flex: 1; }
  .fill-height .md-editor :global(.ProseMirror-menubar-wrapper) { min-height: 0; }
  .md-editor :global(.ProseMirror-menubar) { border-bottom: 1px solid var(--border); background: var(--bg-white); padding: 4px 8px; min-height: 32px; display: flex; flex-wrap: wrap; gap: 2px; align-items: center; }
  .fill-height .md-editor :global(.ProseMirror-menubar) { background: var(--bg-page); border-bottom-color: transparent; padding-left: max(1rem, calc(50% - 364px)); padding-right: max(1rem, calc(50% - 364px)); }
  .md-editor :global(.ProseMirror-menu-active) { background: var(--accent); color: white; border-radius: 2px; }
  .md-editor :global(.ProseMirror-icon) { cursor: pointer; padding: 2px 6px; border-radius: 2px; display: inline-flex; align-items: center; }
  .md-editor :global(.ProseMirror-icon:hover) { background: var(--bg-hover); }
  .md-editor :global(.ProseMirror-icon svg) { fill: currentColor; width: 16px; height: 16px; }
  .md-editor :global(.ProseMirror-menuseparator) { border-right: 1px solid var(--border); height: 16px; margin: 0 4px; }
  .md-editor :global(.ProseMirror-tooltip) { background: #333; color: white; font-size: 11px; padding: 3px 6px; border-radius: 3px; }
  .md-editor :global(.ProseMirror ::selection) { background: rgba(95, 155, 101, 0.25); }
</style>
