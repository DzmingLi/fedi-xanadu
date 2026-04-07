<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState, type Transaction } from 'prosemirror-state';
  import { EditorView, type NodeView } from 'prosemirror-view';
  import { Schema, type Node as PNode } from 'prosemirror-model';
  import { schema as basicSchema } from 'prosemirror-schema-basic';
  import { addListNodes } from 'prosemirror-schema-list';
  import { exampleSetup } from 'prosemirror-example-setup';
  import { tableNodes, columnResizing, tableEditing, goToNextCell } from 'prosemirror-tables';
  import { keymap } from 'prosemirror-keymap';
  import { inputRules, InputRule, textblockTypeInputRule, wrappingInputRule } from 'prosemirror-inputrules';
  import { t } from '../i18n/index.svelte';

  let { value = $bindable(''), placeholder = '', fillHeight = false }: {
    value: string; placeholder?: string; fillHeight?: boolean;
  } = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  let updating = false;

  // ── Schema ──────────────────────────────────────────────────────────────
  const baseNodes = addListNodes(basicSchema.spec.nodes, 'paragraph block*', 'block');
  const tNodes = tableNodes({ tableGroup: 'block', cellContent: 'block+', cellAttributes: {} });

  const typstSchema = new Schema({
    nodes: (baseNodes as any).append(tNodes).append({
      math_inline: {
        group: 'inline',
        inline: true,
        atom: true,
        attrs: { formula: { default: '' } },
        parseDOM: [{ tag: 'span[data-math]', getAttrs: (d: any) => ({ formula: d.dataset.math }) }],
        toDOM: (n: PNode) => ['span', { 'data-math': n.attrs.formula, class: 'typst-math-inline' }],
      },
      math_block: {
        group: 'block',
        atom: true,
        attrs: { formula: { default: '' } },
        parseDOM: [{ tag: 'div[data-math-block]', getAttrs: (d: any) => ({ formula: d.dataset.mathBlock }) }],
        toDOM: (n: PNode) => ['div', { 'data-math-block': n.attrs.formula, class: 'typst-math-block' }],
      },
    }),
    marks: basicSchema.spec.marks,
  });

  // ── Math rendering cache (formula → rendered HTML) ──────────────────────
  const mathCache = new Map<string, string>();

  async function fetchMathHtml(formula: string, display: boolean): Promise<string> {
    const key = `${display ? 'B' : 'I'}:${formula}`;
    if (mathCache.has(key)) return mathCache.get(key)!;
    try {
      const res = await fetch('/api/render/typst-snippet', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ formula, display }),
      });
      if (res.ok) {
        const { html } = await res.json();
        mathCache.set(key, html);
        return html;
      }
    } catch {}
    // Fallback: show source
    return display ? `<span class="math-fallback">$ ${formula} $</span>` : `<span class="math-fallback">$${formula}$</span>`;
  }

  // ── Math NodeView — renders via Typst backend, click-to-edit ─────────────
  class MathNodeView implements NodeView {
    dom: HTMLElement;
    private _display: boolean;
    private _formula: string;
    private _editing = false;
    private _editorEl: HTMLTextAreaElement | HTMLInputElement | null = null;
    private _view: EditorView | null = null;
    private _getPos: (() => number | undefined) | boolean;

    constructor(node: PNode, editorView: EditorView, getPos: (() => number | undefined) | boolean) {
      this._display = node.type.name === 'math_block';
      this._formula = node.attrs.formula;
      this._getPos = getPos;
      this._view = editorView;

      this.dom = document.createElement(this._display ? 'div' : 'span');
      this.dom.className = this._display ? 'typst-math-block-view' : 'typst-math-inline-view';
      this.dom.contentEditable = 'false';
      this._renderMath();
    }

    private _renderMath() {
      const f = this._formula;
      const d = this._display;
      const dom = this.dom;
      // Show source immediately, then replace with rendered HTML
      dom.innerHTML = d
        ? `<span class="math-source">$ ${f} $</span>`
        : `<span class="math-source">$${f}$</span>`;
      fetchMathHtml(f, d).then(html => {
        if (!this._editing && this._formula === f) {
          dom.innerHTML = html;
        }
      });
    }

    private _startEditing() {
      if (this._editing) return;
      this._editing = true;
      this.dom.classList.add('editing');
      this.dom.innerHTML = '';

      if (this._display) {
        const ta = document.createElement('textarea');
        ta.className = 'math-edit-input';
        ta.value = this._formula;
        ta.rows = Math.max(2, this._formula.split('\n').length + 1);
        ta.addEventListener('keydown', e => {
          if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); this._commitEdit(ta.value); }
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); e.stopPropagation(); this._commitEdit(ta.value); }
          e.stopPropagation(); // Don't let PM see keystrokes
        });
        ta.addEventListener('blur', () => this._commitEdit(ta.value));
        this.dom.appendChild(ta);
        this._editorEl = ta;
        setTimeout(() => ta.focus(), 0);
      } else {
        const inp = document.createElement('input');
        inp.type = 'text';
        inp.className = 'math-edit-input';
        inp.value = this._formula;
        inp.addEventListener('keydown', e => {
          if (e.key === 'Enter' || e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); this._commitEdit(inp.value); }
          e.stopPropagation();
        });
        inp.addEventListener('blur', () => this._commitEdit(inp.value));
        this.dom.appendChild(inp);
        this._editorEl = inp;
        setTimeout(() => { inp.focus(); inp.select(); }, 0);
      }
    }

    private _commitEdit(newFormula: string) {
      if (!this._editing) return;
      this._editing = false;
      this._editorEl = null;
      this.dom.classList.remove('editing');
      const trimmed = newFormula.trim();
      if (trimmed !== this._formula && this._view) {
        const pos = typeof this._getPos === 'function' ? this._getPos() : undefined;
        if (pos !== undefined) {
          const nodeType = this._display ? typstSchema.nodes.math_block : typstSchema.nodes.math_inline;
          const tr = this._view.state.tr.setNodeMarkup(pos, nodeType, { formula: trimmed });
          this._view.dispatch(tr);
          return; // update() will be called, which re-renders
        }
      }
      this._formula = trimmed || this._formula;
      this._renderMath();
    }

    update(node: PNode) {
      if (node.type !== (this._display ? typstSchema.nodes.math_block : typstSchema.nodes.math_inline)) return false;
      const newFormula = node.attrs.formula;
      if (newFormula !== this._formula) {
        this._formula = newFormula;
        if (!this._editing) this._renderMath();
      }
      return true;
    }

    selectNode() {
      this.dom.classList.add('selected');
      this._startEditing();
    }
    deselectNode() {
      this.dom.classList.remove('selected');
      if (this._editing && this._editorEl) {
        this._commitEdit(this._editorEl.value);
      }
    }
    stopEvent(e: Event) {
      // Let our edit input handle events
      return this._editing && (e.target === this._editorEl || this.dom.contains(e.target as Node));
    }
    ignoreMutation() { return true; }
    destroy() { this._view = null; }
  }

  // ── Typst input rules ────────────────────────────────────────────────────
  const typstRules = inputRules({
    rules: [
      // = Title, == Title, etc. → headings
      textblockTypeInputRule(
        /^(={1,6})\s$/,
        typstSchema.nodes.heading,
        (m: RegExpMatchArray) => ({ level: m[1].length }),
      ),
      // + item → ordered list
      wrappingInputRule(/^\+\s$/, typstSchema.nodes.ordered_list),
      // $formula$ → math_inline node
      new InputRule(/\$([^$\n]{1,200})\$$/, (state, match, start, end) => {
        const formula = match[1].trim();
        if (!formula) return null;
        return state.tr.replaceWith(start, end,
          typstSchema.nodes.math_inline.create({ formula }));
      }),
    ],
  });

  // ── Serializer: ProseMirror doc → Typst ─────────────────────────────────
  function serializeInline(node: PNode): string {
    if (node.type.name === 'math_inline') return `$${node.attrs.formula}$`;
    if (node.isText) {
      let t = node.text ?? '';
      const marks = node.marks.map(m => m.type.name);
      if (marks.includes('strong')) t = `*${t}*`;
      if (marks.includes('em'))     t = `_${t}_`;
      if (marks.includes('code'))   t = '`' + t + '`';
      return t;
    }
    let out = '';
    node.forEach(c => { out += serializeInline(c); });
    return out;
  }

  function serializeBlock(node: PNode): string {
    switch (node.type.name) {
      case 'paragraph': {
        let t = '';
        node.forEach(c => { t += serializeInline(c); });
        return t + '\n';
      }
      case 'heading': {
        let t = '';
        node.forEach(c => { t += serializeInline(c); });
        return '='.repeat(node.attrs.level) + ' ' + t + '\n';
      }
      case 'bullet_list': {
        let out = '';
        node.forEach(item => {
          let t = '';
          item.forEach(c => { if (c.type.name === 'paragraph') c.forEach(i => { t += serializeInline(i); }); });
          out += `- ${t}\n`;
        });
        return out;
      }
      case 'ordered_list': {
        let out = '';
        node.forEach(item => {
          let t = '';
          item.forEach(c => { if (c.type.name === 'paragraph') c.forEach(i => { t += serializeInline(i); }); });
          out += `+ ${t}\n`;
        });
        return out;
      }
      case 'blockquote': {
        let t = '';
        node.forEach(c => { if (c.type.name === 'paragraph') c.forEach(i => { t += serializeInline(i); }); });
        return `#quote[${t}]\n`;
      }
      case 'code_block':
        return '```\n' + node.textContent + '\n```\n';
      case 'horizontal_rule':
        return '---\n';
      case 'math_block':
        return '$\n' + node.attrs.formula + '\n$\n';
      case 'table': {
        const firstRow = node.firstChild;
        if (!firstRow) return '';
        const cols = firstRow.childCount;
        const cells: string[] = [];
        node.forEach(row => {
          row.forEach(cell => {
            cells.push('[' + cell.textContent.replace(/]/g, '\\]') + ']');
          });
        });
        return `#table(columns: ${cols},\n  ${cells.join(', ')},\n)\n`;
      }
      default: {
        let out = '';
        node.forEach(c => { out += serializeBlock(c); });
        return out;
      }
    }
  }

  function serializeTypst(doc: PNode): string {
    let out = '';
    doc.forEach(block => { out += serializeBlock(block) + '\n'; });
    return out.replace(/\n{3,}/g, '\n\n').trimEnd() + '\n';
  }

  // ── Parser: Typst source → ProseMirror doc ───────────────────────────────
  function parseInline(text: string): PNode[] {
    const nodes: PNode[] = [];
    let i = 0, plain = '';
    const flush = () => {
      if (plain) { nodes.push(typstSchema.text(plain)); plain = ''; }
    };
    while (i < text.length) {
      const rest = text.slice(i);
      const mathM = rest.match(/^\$([^$\n]+)\$/);
      if (mathM) { flush(); nodes.push(typstSchema.nodes.math_inline.create({ formula: mathM[1] })); i += mathM[0].length; continue; }
      const boldM = rest.match(/^\*([^*\n]+)\*/);
      if (boldM) { flush(); nodes.push(typstSchema.text(boldM[1], [typstSchema.marks.strong.create()])); i += boldM[0].length; continue; }
      const emM = rest.match(/^_([^_\n]+)_/);
      if (emM) { flush(); nodes.push(typstSchema.text(emM[1], [typstSchema.marks.em.create()])); i += emM[0].length; continue; }
      const codeM = rest.match(/^`([^`\n]+)`/);
      if (codeM) { flush(); nodes.push(typstSchema.text(codeM[1], [typstSchema.marks.code.create()])); i += codeM[0].length; continue; }
      plain += text[i++];
    }
    flush();
    return nodes.length ? nodes : [typstSchema.text(' ')];
  }

  function parseTypst(text: string): PNode {
    try {
      const blocks: PNode[] = [];
      const lines = text.split('\n');
      let i = 0;

      while (i < lines.length) {
        const line = lines[i];
        if (!line.trim()) { i++; continue; }

        // Heading
        const hm = line.match(/^(={1,6})\s+(.*)/);
        if (hm) {
          blocks.push(typstSchema.nodes.heading.create({ level: hm[1].length }, parseInline(hm[2])));
          i++; continue;
        }
        // Bullet list
        if (/^- /.test(line)) {
          const items: PNode[] = [];
          while (i < lines.length && /^- /.test(lines[i])) {
            items.push(typstSchema.nodes.list_item.create(null,
              [typstSchema.nodes.paragraph.create(null, parseInline(lines[i].slice(2)))]));
            i++;
          }
          blocks.push(typstSchema.nodes.bullet_list.create(null, items));
          continue;
        }
        // Ordered list
        if (/^\+ /.test(line)) {
          const items: PNode[] = [];
          while (i < lines.length && /^\+ /.test(lines[i])) {
            items.push(typstSchema.nodes.list_item.create(null,
              [typstSchema.nodes.paragraph.create(null, parseInline(lines[i].slice(2)))]));
            i++;
          }
          blocks.push(typstSchema.nodes.ordered_list.create(null, items));
          continue;
        }
        // Code block
        if (line.trim() === '```') {
          i++;
          const codeLines: string[] = [];
          while (i < lines.length && lines[i].trim() !== '```') codeLines.push(lines[i++]);
          if (i < lines.length) i++;
          blocks.push(typstSchema.nodes.code_block.create(null,
            codeLines.length ? [typstSchema.text(codeLines.join('\n'))] : []));
          continue;
        }
        // Display math
        if (line.trim() === '$') {
          i++;
          const fLines: string[] = [];
          while (i < lines.length && lines[i].trim() !== '$') fLines.push(lines[i++]);
          if (i < lines.length) i++;
          blocks.push(typstSchema.nodes.math_block.create({ formula: fLines.join('\n') }));
          continue;
        }
        // Horizontal rule
        if (line.trim() === '---') {
          blocks.push(typstSchema.nodes.horizontal_rule.create());
          i++; continue;
        }
        // #quote[text]
        const qm = line.match(/^#quote\[(.+)\]$/);
        if (qm) {
          blocks.push(typstSchema.nodes.blockquote.create(null,
            [typstSchema.nodes.paragraph.create(null, parseInline(qm[1]))]));
          i++; continue;
        }
        // Paragraph (collect until blank line or block start)
        const paraLines: string[] = [];
        while (i < lines.length) {
          const l = lines[i];
          if (!l.trim()) break;
          if (/^(={1,6}\s|```$|\$\s*$|---$|#quote\[)/.test(l)) break;
          if (/^[+-] /.test(l) && paraLines.length > 0) break;
          paraLines.push(l);
          i++;
        }
        if (paraLines.length) {
          blocks.push(typstSchema.nodes.paragraph.create(null, parseInline(paraLines.join('\n'))));
        }
      }
      if (!blocks.length) blocks.push(typstSchema.nodes.paragraph.create());
      return typstSchema.nodes.doc.create(null, blocks);
    } catch {
      return typstSchema.topNodeType.createAndFill()!;
    }
  }

  function insertTable() {
    if (!view) return;
    const { state, dispatch } = view;
    const { table, table_row, table_cell, table_header } = typstSchema.nodes;
    const headerCells = [0, 1, 2].map(() => table_header.createAndFill()!);
    const bodyCells   = [0, 1, 2].map(() => table_cell.createAndFill()!);
    dispatch(state.tr.replaceSelectionWith(
      table.create(null, [table_row.create(null, headerCells), table_row.create(null, bodyCells)])
    ));
    view.focus();
  }

  // ── Mount ─────────────────────────────────────────────────────────────────
  onMount(() => {
    const state = EditorState.create({
      doc: parseTypst(value),
      plugins: [
        typstRules,                           // Typst input rules (higher priority)
        ...exampleSetup({ schema: typstSchema }),
        columnResizing(),
        tableEditing(),
        keymap({ 'Tab': goToNextCell(1), 'Shift-Tab': goToNextCell(-1) }),
      ],
    });

    view = new EditorView(container, {
      state,
      nodeViews: {
        math_inline: (node, editorView, getPos) => new MathNodeView(node, editorView, getPos),
        math_block:  (node, editorView, getPos) => new MathNodeView(node, editorView, getPos),
      },
      dispatchTransaction(tr: Transaction) {
        if (!view) return;
        const newState = view.state.apply(tr);
        view.updateState(newState);
        if (!updating && tr.docChanged) {
          updating = true;
          try { value = serializeTypst(newState.doc); } catch {}
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

    return () => { view?.destroy(); view = null; };
  });

  // Sync external value changes into editor
  $effect(() => {
    if (!view || updating) return;
    const current = serializeTypst(view.state.doc);
    if (value !== current) {
      updating = true;
      const newDoc = parseTypst(value);
      const tr = view.state.tr.replaceWith(0, view.state.doc.content.size, newDoc.content);
      view.dispatch(tr);
      updating = false;
    }
  });
</script>

<div class="md-editor-wrapper" class:fill-height={fillHeight}>
  <div class="md-editor" bind:this={container}></div>
  {#if !value && placeholder}
    <div class="md-placeholder">{placeholder}</div>
  {/if}
</div>

<style>
  /* Reuse the same layout/visual styles as MarkdownEditor */
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

  .md-editor {
    flex: 1;
    min-height: 300px;
    overflow-y: auto;
    position: relative;
  }
  .fill-height .md-editor {
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

  .md-editor :global(.ProseMirror p) { margin: 1em 0; overflow-wrap: break-word; }
  .md-editor :global(.ProseMirror h1) { font-family: var(--font-serif); font-size: 2rem; font-weight: 400; margin: 2em 0 0.5em; }
  .md-editor :global(.ProseMirror h2) { font-family: var(--font-serif); font-size: 1.6rem; font-weight: 400; margin: 1.75em 0 0.5em; padding-bottom: 0.25em; border-bottom: 1px solid var(--border); }
  .md-editor :global(.ProseMirror h3) { font-family: var(--font-serif); font-size: 1.2rem; font-weight: 600; margin: 1.5em 0 0.4em; }

  /* Typst heading level prefix */
  .md-editor :global(.ProseMirror h1)::before { content: '= '; font-family: var(--font-mono, monospace); font-size: 0.55em; font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }
  .md-editor :global(.ProseMirror h2)::before { content: '== '; font-family: var(--font-mono, monospace); font-size: 0.6em; font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }
  .md-editor :global(.ProseMirror h3)::before { content: '=== '; font-family: var(--font-mono, monospace); font-size: 0.7em; font-weight: 400; color: var(--text-hint); vertical-align: middle; margin-right: 0.1em; }

  .md-editor :global(.ProseMirror code) { font-size: 0.9em; padding: 0.15em 0.35em; background: var(--bg-gray, #f5f5f5); border-radius: 3px; }
  .md-editor :global(.ProseMirror pre) { overflow-x: auto; padding: 1em; margin: 1em 0; background: var(--bg-gray, #f5f5f5); border-radius: 4px; font-size: 0.9em; line-height: 1.5; }
  .md-editor :global(.ProseMirror pre code) { padding: 0; background: none; }
  .md-editor :global(.ProseMirror blockquote) { margin: 1em 0; padding: 0.5em 1em; border-left: 3px solid var(--border-strong); color: var(--text-secondary); }
  .md-editor :global(.ProseMirror ul), .md-editor :global(.ProseMirror ol) { padding-left: 1.5em; margin: 0.75em 0; }
  .md-editor :global(.ProseMirror li) { margin: 0.25em 0; }
  .md-editor :global(.ProseMirror hr) { border: none; border-top: 1px solid var(--border); margin: 1em 0; }
  .md-editor :global(.ProseMirror img) { max-width: 100%; height: auto; }
  .md-editor :global(.ProseMirror-focused) { outline: none; }

  /* Math nodes — rendered by Typst backend, click-to-edit */
  .md-editor :global(.typst-math-inline-view) {
    display: inline-block;
    vertical-align: middle;
    cursor: pointer;
    border-radius: 3px;
    padding: 0 2px;
  }
  .md-editor :global(.typst-math-block-view) {
    display: block;
    border-radius: 4px;
    margin: 0.75em 0;
    text-align: center;
    cursor: pointer;
  }
  .md-editor :global(.typst-math-inline-view.selected),
  .md-editor :global(.typst-math-block-view.selected) {
    outline: 2px solid var(--accent, #4a7);
    outline-offset: 2px;
    background: rgba(95, 155, 101, 0.06);
  }
  /* Fallback source display while loading */
  .md-editor :global(.math-source),
  .md-editor :global(.math-fallback) {
    font-family: var(--font-mono, monospace);
    font-size: 0.88em;
    color: #2a6b4a;
    background: rgba(42, 107, 74, 0.07);
    border-radius: 3px;
    padding: 0 4px;
  }
  /* Editing state — inline input */
  .md-editor :global(.typst-math-inline-view.editing),
  .md-editor :global(.typst-math-block-view.editing) {
    outline: 2px solid var(--accent, #4a7);
    outline-offset: 2px;
    background: rgba(95, 155, 101, 0.06);
    display: inline-flex;
    align-items: center;
  }
  .md-editor :global(.typst-math-block-view.editing) {
    display: block;
  }
  .md-editor :global(.math-edit-input) {
    font-family: var(--font-mono, monospace);
    font-size: 0.9em;
    border: none;
    outline: none;
    background: transparent;
    color: #2a6b4a;
    padding: 2px 4px;
    min-width: 6ch;
    width: auto;
    resize: none;
    line-height: 1.4;
  }
  .md-editor :global(textarea.math-edit-input) {
    display: block;
    width: 100%;
    min-height: 2em;
  }
  /* Typst-rendered HTML (SVG/MathML from typst-ts) */
  .md-editor :global(.typst-math-inline-view svg),
  .md-editor :global(.typst-math-block-view svg) {
    vertical-align: middle;
    max-width: 100%;
    height: auto;
  }

  /* Table */
  .md-editor :global(table) { border-collapse: collapse; margin: 1.25em auto; font-size: 0.95em; width: auto; }
  .md-editor :global(th), .md-editor :global(td) { border: 1px solid var(--border-strong); padding: 0.5em 0.875em; min-width: 60px; text-align: left; vertical-align: top; position: relative; }
  .md-editor :global(th) { font-weight: 600; font-family: var(--font-sans); font-size: 0.85em; text-transform: uppercase; letter-spacing: 0.03em; }
  .md-editor :global(.selectedCell) { background: rgba(95, 155, 101, 0.15); }
  .md-editor :global(.column-resize-handle) { position: absolute; right: -2px; top: 0; bottom: 0; width: 4px; background: var(--accent, #4a7); cursor: col-resize; z-index: 20; }

  /* Menu bar */
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
