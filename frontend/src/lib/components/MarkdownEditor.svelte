<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState, type Transaction } from 'prosemirror-state';
  import { EditorView } from 'prosemirror-view';
  import { Schema } from 'prosemirror-model';
  import { schema as basicSchema } from 'prosemirror-schema-basic';
  import { addListNodes } from 'prosemirror-schema-list';
  import { exampleSetup } from 'prosemirror-example-setup';
  import { defaultMarkdownParser, defaultMarkdownSerializer, MarkdownParser, MarkdownSerializer } from 'prosemirror-markdown';
  import { tableNodes, columnResizing, tableEditing, goToNextCell, addColumnAfter, addColumnBefore, deleteColumn, addRowAfter, addRowBefore, deleteRow, mergeCells, splitCell, toggleHeaderRow, toggleHeaderColumn, toggleHeaderCell } from 'prosemirror-tables';
  import { keymap } from 'prosemirror-keymap';
  import { t } from '../i18n/index.svelte';

  let { value = $bindable(''), placeholder = '', fillHeight = false }: { value: string; placeholder?: string; fillHeight?: boolean } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  let updating = false;
  let fullscreen = $state(false);

  // Build schema with table nodes
  const baseNodes = addListNodes(basicSchema.spec.nodes, 'paragraph block*', 'block');
  const tNodes = tableNodes({
    tableGroup: 'block',
    cellContent: 'block+',
    cellAttributes: {},
  });
  const mdSchema = new Schema({
    nodes: (baseNodes as any)
      .append(tNodes),
    marks: basicSchema.spec.marks,
  });

  const mdParser = new MarkdownParser(mdSchema, defaultMarkdownParser.tokenizer, {
    ...defaultMarkdownParser.tokens,
  });

  // Extended serializer that handles table nodes (defaultMarkdownSerializer throws on unknown nodes)
  const mdSerializer = new MarkdownSerializer(
    {
      ...defaultMarkdownSerializer.nodes,
      table(state, node) {
        const rows: string[][] = [];
        node.forEach(row => {
          const cells: string[] = [];
          row.forEach(cell => {
            cells.push(cell.textContent.replace(/\|/g, '\\|').replace(/\n/g, ' '));
          });
          rows.push(cells);
        });
        if (rows.length === 0) return;
        state.write('| ' + rows[0].join(' | ') + ' |\n');
        state.write('| ' + rows[0].map(() => '---').join(' | ') + ' |\n');
        for (let i = 1; i < rows.length; i++) {
          state.write('| ' + rows[i].join(' | ') + ' |\n');
        }
        state.write('\n');
      },
      table_row() {},
      table_cell() {},
      table_header() {},
    },
    defaultMarkdownSerializer.marks,
  );

  function parseMarkdown(text: string) {
    try {
      return mdParser.parse(text) ?? mdSchema.topNodeType.createAndFill()!;
    } catch {
      return mdSchema.topNodeType.createAndFill()!;
    }
  }

  function insertTable() {
    if (!view) return;
    const { state, dispatch } = view;
    const { table, table_row, table_cell, table_header } = mdSchema.nodes;
    const headerCells = [0, 1, 2].map(() => table_header.createAndFill()!);
    const bodyCells = [0, 1, 2].map(() => table_cell.createAndFill()!);
    const headerRow = table_row.create(null, headerCells);
    const bodyRow = table_row.create(null, bodyCells);
    const tbl = table.create(null, [headerRow, bodyRow]);
    dispatch(state.tr.replaceSelectionWith(tbl));
    view.focus();
  }

  function toggleFullscreen() {
    fullscreen = !fullscreen;
    // Re-focus editor after toggling
    setTimeout(() => view?.focus(), 50);
  }

  onMount(() => {
    const state = EditorState.create({
      doc: parseMarkdown(value),
      plugins: [
        ...exampleSetup({ schema: mdSchema }),
        columnResizing(),
        tableEditing(),
        keymap({
          'Tab': goToNextCell(1),
          'Shift-Tab': goToNextCell(-1),
        }),
      ],
    });

    view = new EditorView(container, {
      state,
      dispatchTransaction(tr: Transaction) {
        if (!view) return;
        const newState = view.state.apply(tr);
        view.updateState(newState);
        if (!updating && tr.docChanged) {
          updating = true;
          try { value = mdSerializer.serialize(newState.doc); } catch {}
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

    // Escape exits fullscreen
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && fullscreen) {
        fullscreen = false;
      }
    };
    document.addEventListener('keydown', handleKey);

    return () => {
      document.removeEventListener('keydown', handleKey);
      view?.destroy();
      view = null;
    };
  });

  // Sync external value changes into the editor
  $effect(() => {
    if (!view || updating) return;
    const current = mdSerializer.serialize(view.state.doc);
    if (value !== current) {
      updating = true;
      const newDoc = parseMarkdown(value);
      const tr = view.state.tr.replaceWith(0, view.state.doc.content.size, newDoc.content);
      view.dispatch(tr);
      updating = false;
    }
  });
</script>

<div class="md-editor-wrapper" class:fullscreen class:fill-height={fillHeight}>
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
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
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

  .fill-height .md-editor {
    min-height: 0;
  }

  .fullscreen .md-editor {
    min-height: 0;
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

  /*
   * ProseMirror editing area — mirrors .content from app.css
   * so the editor looks identical to the rendered article.
   */
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
    text-align: justify;
    hyphens: auto;
  }

  .fill-height .md-editor :global(.ProseMirror) {
    min-height: 0;
    max-width: 760px;
    margin: 0 auto;
    padding: 2rem 1rem;
  }

  .fullscreen .md-editor :global(.ProseMirror) {
    min-height: 0;
    height: 100%;
    max-width: 760px;
    margin: 0 auto;
    padding: 2rem 1rem;
  }

  .md-editor :global(.ProseMirror p) { margin: 1em 0; overflow-wrap: break-word; }

  .md-editor :global(.ProseMirror h1) {
    font-family: var(--font-serif);
    font-size: 2rem;
    font-weight: 400;
    margin: 2em 0 0.5em;
  }
  .md-editor :global(.ProseMirror h2) {
    font-family: var(--font-serif);
    font-size: 1.6rem;
    font-weight: 400;
    margin: 1.75em 0 0.5em;
    padding-bottom: 0.25em;
    border-bottom: 1px solid var(--border);
  }
  .md-editor :global(.ProseMirror h3) {
    font-family: var(--font-serif);
    font-size: 1.2rem;
    font-weight: 600;
    margin: 1.5em 0 0.4em;
  }

  .md-editor :global(.ProseMirror code) {
    font-size: 0.9em;
    padding: 0.15em 0.35em;
    background: var(--bg-gray, #f5f5f5);
    border-radius: 3px;
  }

  .md-editor :global(.ProseMirror pre) {
    overflow-x: auto;
    padding: 1em;
    margin: 1em 0;
    background: var(--bg-gray, #f5f5f5);
    border-radius: 4px;
    font-size: 0.9em;
    line-height: 1.5;
  }
  .md-editor :global(.ProseMirror pre code) { padding: 0; background: none; }

  .md-editor :global(.ProseMirror blockquote) {
    margin: 1em 0;
    padding: 0.5em 1em;
    border-left: 3px solid var(--border-strong);
    color: var(--text-secondary);
  }

  .md-editor :global(.ProseMirror ul),
  .md-editor :global(.ProseMirror ol) { padding-left: 1.5em; margin: 0.75em 0; }
  .md-editor :global(.ProseMirror li) { margin: 0.25em 0; }

  .md-editor :global(.ProseMirror hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 1em 0;
  }

  .md-editor :global(.ProseMirror img) { max-width: 100%; height: auto; }

  .md-editor :global(.ProseMirror-focused) { outline: none; }

  /* Table styles — mirrors .content table from app.css */
  .md-editor :global(table) {
    border-collapse: collapse;
    margin: 1.25em auto;
    font-size: 0.95em;
    width: auto;
    overflow: auto;
  }
  .md-editor :global(th),
  .md-editor :global(td) {
    border: 1px solid var(--border-strong);
    padding: 0.5em 0.875em;
    min-width: 60px;
    text-align: left;
    vertical-align: top;
    position: relative;
  }
  .md-editor :global(th) {
    font-weight: 600;
    font-family: var(--font-sans);
    font-size: 0.85em;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .md-editor :global(.selectedCell) {
    background: rgba(95, 155, 101, 0.15);
  }
  .md-editor :global(.column-resize-handle) {
    position: absolute;
    right: -2px;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--accent, #4a7);
    cursor: col-resize;
    z-index: 20;
  }

  /* Menu bar (from prosemirror-example-setup) */
  .md-editor :global(.ProseMirror-menubar-wrapper) { border: none; display: flex; flex-direction: column; flex: 1; }
  .fill-height .md-editor :global(.ProseMirror-menubar-wrapper) { min-height: 0; }

  .md-editor :global(.ProseMirror-menubar) {
    border-bottom: 1px solid var(--border);
    background: var(--bg-white);
    padding: 4px 8px;
    min-height: 32px;
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    align-items: center;
  }

  .fill-height .md-editor :global(.ProseMirror-menubar) {
    background: var(--bg-page);
    border-bottom-color: transparent;
    padding-left: max(1rem, calc(50% - 364px));
    padding-right: max(1rem, calc(50% - 364px));
  }

  .md-editor :global(.ProseMirror-menu-active) {
    background: var(--accent);
    color: white;
    border-radius: 2px;
  }

  .md-editor :global(.ProseMirror-icon) {
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 2px;
    display: inline-flex;
    align-items: center;
  }
  .md-editor :global(.ProseMirror-icon:hover) { background: var(--bg-hover); }
  .md-editor :global(.ProseMirror-icon svg) { fill: currentColor; width: 16px; height: 16px; }

  .md-editor :global(.ProseMirror-menuseparator) {
    border-right: 1px solid var(--border);
    height: 16px;
    margin: 0 4px;
  }

  .md-editor :global(.ProseMirror-tooltip) {
    background: #333;
    color: white;
    font-size: 11px;
    padding: 3px 6px;
    border-radius: 3px;
  }

  .md-editor :global(.ProseMirror ::selection) {
    background: rgba(95, 155, 101, 0.25);
  }
</style>
