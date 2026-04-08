<script module lang="ts">
  import { Schema, type Node as PNode } from 'prosemirror-model';
  import { schema as basicSchema } from 'prosemirror-schema-basic';
  import { addListNodes } from 'prosemirror-schema-list';
  import { tableNodes } from 'prosemirror-tables';
  import { inputRules, textblockTypeInputRule, wrappingInputRule } from 'prosemirror-inputrules';
  import { Plugin, TextSelection } from 'prosemirror-state';
  import { type NodeView, Decoration, DecorationSet } from 'prosemirror-view';
  import type { EditorView } from 'prosemirror-view';

  // ── Schema ───────────────────────────────────────────────────────────────────
  // math_inline: atom node — formula stored in attrs.formula.
  //   Editing via a floating <input> overlay (avoids Firefox inline-NodeView
  //   cursor reverse-mapping bug that prevents ProseMirror from tracking cursor
  //   position inside inline contentDOM).
  // math_block:  content node — formula stored as text content, contentDOM works
  //   fine for block-level nodes in all browsers.
  const baseNodes = addListNodes(basicSchema.spec.nodes, 'paragraph block*', 'block');
  export const typstSchema = new Schema({
    nodes: (baseNodes as any)
      .append(tableNodes({ tableGroup: 'block', cellContent: 'block+', cellAttributes: {} }))
      .append({
        math_inline: {
          group: 'inline', inline: true,
          atom: true,
          attrs: { formula: { default: '' } },
          marks: '',
          parseDOM: [{ tag: 'span.typst-math-inline', getAttrs: (dom: any) => ({ formula: dom.dataset?.formula ?? '' }) }],
          toDOM: (node: PNode) => ['span', { class: 'typst-math-inline', 'data-formula': node.attrs.formula }],
        },
        math_block: {
          group: 'block',
          content: 'text*',
          marks: '',
          parseDOM: [{ tag: 'div.typst-math-block' }],
          toDOM: () => ['div', { class: 'typst-math-block' }, 0],
        },
      }),
    marks: basicSchema.spec.marks,
  });

  // ── Math rendering cache ─────────────────────────────────────────────────────
  const mathCache = new Map<string, string>();

  // Typst wraps all output in <p>…</p>. Strip it so we can safely set
  // innerHTML on either a <span> (inline) or <div> (block) without invalid
  // nesting — a <p> inside a <span> causes the browser to eject the <span>
  // content entirely, making the rendered element appear empty.
  function stripPWrapper(html: string): string {
    const tmp = document.createElement('div');
    tmp.innerHTML = html;
    const first = tmp.firstElementChild;
    if (first && first.tagName === 'P') return first.innerHTML;
    return html;
  }

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
        const stripped = stripPWrapper(html);
        mathCache.set(key, stripped);
        return stripped;
      }
      console.warn('[math] render API returned', res.status, 'for formula:', formula);
    } catch (err) {
      console.warn('[math] render API fetch failed:', err);
    }
    return display
      ? `<span class="math-fallback">$ ${formula} $</span>`
      : `<span class="math-fallback">$${formula}$</span>`;
  }

  // ── MathInlineNodeView ───────────────────────────────────────────────────────
  // Atom node view for math_inline. Formula stored in node.attrs.formula.
  // Editing via a floating <input> that appears on click or on creation of an
  // empty node. Commits formula via tr.setNodeMarkup on Enter or blur.
  export class MathInlineNodeView implements NodeView {
    dom: HTMLElement;
    private _view: EditorView;
    private _getPos: () => number | undefined;
    private _formula: string;
    private _renderEl: HTMLElement;
    private _inputEl: HTMLInputElement | null = null;

    constructor(node: PNode, view: EditorView, getPos: any) {
      this._view = view;
      this._getPos = getPos;
      this._formula = node.attrs.formula ?? '';

      this.dom = document.createElement('span');
      this.dom.className = 'typst-math-inline-view';

      this._renderEl = document.createElement('span');
      this._renderEl.className = 'math-rendered';
      this.dom.appendChild(this._renderEl);

      this._render();

      // Auto-open input for newly created empty nodes
      if (!this._formula) {
        requestAnimationFrame(() => this._openInput());
      }

      this.dom.addEventListener('click', (e) => {
        e.stopPropagation();
        this._openInput();
      });
    }

    update(node: PNode) {
      if (node.type !== typstSchema.nodes.math_inline) return false;
      const newFormula = node.attrs.formula ?? '';
      if (newFormula !== this._formula) {
        this._formula = newFormula;
        this._render();
      }
      return true;
    }

    private _render() {
      if (!this._formula) {
        this._renderEl.innerHTML = `<span class="math-empty">$ $</span>`;
        return;
      }
      const formula = this._formula;
      this._renderEl.innerHTML = `<span class="math-placeholder">$${formula}$</span>`;
      fetchMathHtml(formula, false).then(html => {
        if (this._formula === formula) this._renderEl.innerHTML = html;
      });
    }

    private _openInput() {
      if (this._inputEl) { this._inputEl.focus(); return; }

      const input = document.createElement('input');
      input.type = 'text';
      input.className = 'math-inline-input';
      input.value = this._formula;
      input.placeholder = 'Typst 公式…';

      const rect = this.dom.getBoundingClientRect();
      const left = Math.min(rect.left, window.innerWidth - 220);
      const top = rect.bottom + 4 + window.scrollY;
      input.style.cssText = [
        'position:fixed',
        `left:${left}px`,
        `top:${rect.bottom + 4}px`,
        'z-index:9999',
        'font-family:monospace',
        'font-size:0.9em',
        'padding:4px 8px',
        'border:2px solid var(--accent,#4a7)',
        'border-radius:4px',
        'background:#fff',
        'color:#000',
        'min-width:200px',
        'box-shadow:0 2px 8px rgba(0,0,0,.2)',
        'outline:none',
      ].join(';');

      document.body.appendChild(input);
      this._inputEl = input;
      this.dom.classList.add('math-focused');
      input.focus();
      input.select();

      let committed = false;

      const commit = () => {
        if (committed) return;
        committed = true;
        const formula = input.value.trim();
        const pos = this._getPos();
        if (pos !== undefined) {
          const { state, dispatch } = this._view;
          if (state.doc.nodeAt(pos)?.type === typstSchema.nodes.math_inline) {
            dispatch(state.tr.setNodeMarkup(pos, undefined, { formula }));
          }
        }
        cleanup();
      };

      const cleanup = () => {
        if (this._inputEl) {
          document.body.removeChild(this._inputEl);
          this._inputEl = null;
        }
        this.dom.classList.remove('math-focused');
      };

      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') { e.preventDefault(); commit(); this._view.focus(); }
        if (e.key === 'Escape') { e.preventDefault(); cleanup(); this._view.focus(); }
        e.stopPropagation();
      });

      input.addEventListener('blur', () => { if (!committed) commit(); });
    }

    stopEvent() { return false; }
    ignoreMutation() { return true; }

    destroy() {
      if (this._inputEl) {
        document.body.removeChild(this._inputEl);
        this._inputEl = null;
      }
    }
  }

  // ── MathNodeView (math_block only) ───────────────────────────────────────────
  // Mirrors the heading NodeView pattern:
  //   cursor inside  → source mode: formula text visible + $ delimiters via CSS
  //   cursor outside → rendered mode: MathML shown, formula text collapsed
  //
  // The mathFocusPlugin below adds a `mathFocused` node decoration when the
  // cursor is inside a math_block; this NodeView reacts to that decoration.
  export class MathNodeView implements NodeView {
    dom: HTMLElement;
    contentDOM: HTMLElement;
    private _renderEl: HTMLElement;
    private _focused = false;
    private _lastFormula = '';

    constructor(node: PNode, _view: EditorView, _getPos: any) {
      this.dom = document.createElement('div');
      this.dom.className = 'typst-math-block-view';

      this.contentDOM = document.createElement('div');
      this.contentDOM.className = 'math-source-text';

      this._renderEl = document.createElement('div');
      this._renderEl.className = 'math-rendered';

      this.dom.appendChild(this.contentDOM);
      this.dom.appendChild(this._renderEl);

      this._applyRendered(node.textContent);
    }

    update(node: PNode, decorations: readonly any[]) {
      if (node.type !== typstSchema.nodes.math_block) return false;

      const focused = decorations.some((d: any) => d.spec?.mathFocused);
      const formula = node.textContent;

      if (focused !== this._focused) {
        this._focused = focused;
        focused ? this._applySource() : this._applyRendered(formula);
      } else if (!focused && formula !== this._lastFormula) {
        this._applyRendered(formula);
      }
      return true;
    }

    private _applySource() {
      this.dom.classList.add('math-focused');
      this.contentDOM.style.cssText = 'display:block;min-height:1.2em';
      this._renderEl.style.display = 'none';
    }

    private _applyRendered(formula: string) {
      this._lastFormula = formula;
      this.dom.classList.remove('math-focused');
      this.contentDOM.style.cssText = 'display:block;height:0;overflow:hidden';
      this._renderEl.style.display = '';
      if (!formula) {
        this._renderEl.innerHTML = `<span class="math-empty">$ $</span>`;
        return;
      }
      this._renderEl.innerHTML = `<span class="math-placeholder">$ ${formula} $</span>`;
      fetchMathHtml(formula, true).then(html => {
        if (!this._focused && this._lastFormula === formula) this._renderEl.innerHTML = html;
      });
    }

    ignoreMutation(mut: MutationRecord | { type: 'selection'; target: Element }) {
      const record = mut as MutationRecord;
      if (record.type === 'attributes') return true;
      const target = record.target;
      if (!target) return false;
      return !(this.contentDOM.contains(target as Node) || target === this.contentDOM);
    }

    destroy() {}
  }

  // ── Math focus plugin ─────────────────────────────────────────────────────────
  // Adds a mathFocused node decoration to the math node containing the cursor,
  // mirroring the heading focusPlugin in ProseEditorBase.
  const mathFocusPlugin = new Plugin({
    props: {
      decorations(state) {
        const from = state.selection.$from;
        for (let d = from.depth; d >= 0; d--) {
          const n = from.node(d);
          if (n.type.name === 'math_inline' || n.type.name === 'math_block') {
            const start = from.before(d);
            return DecorationSet.create(state.doc, [
              Decoration.node(start, start + n.nodeSize, {}, { mathFocused: true }),
            ]);
          }
        }
        return DecorationSet.empty;
      },
    },
  });

  // ── Math creation via keydown ────────────────────────────────────────────────
  // $formula$  → math_inline  (type closing $, cursor placed after node)
  // "$ " + $   → math_block   (dollar-space in empty para, then $; cursor inside)
  const mathKeyPlugin = new Plugin({
    props: {
      handleKeyDown(view, e) {
        if (e.key !== '$' || e.isComposing) return false;
        const { state } = view;
        if (!(state.selection instanceof TextSelection)) return false;
        const cursor = state.selection.$cursor;
        if (!cursor || cursor.parent.type.name !== 'paragraph') return false;

        const textBefore = cursor.parent.textBetween(0, cursor.parentOffset, null, '\ufffc');

        // "$ " (dollar + space) in empty paragraph → empty display math block, cursor inside
        if (/^\$\s+$/.test(textBefore)) {
          const paraStart = cursor.before();
          const paraEnd   = paraStart + cursor.parent.nodeSize;
          const block = typstSchema.nodes.math_block.create(null, []);
          const tr = state.tr.replaceWith(paraStart, paraEnd, block);
          view.dispatch(tr.setSelection(TextSelection.create(tr.doc, tr.mapping.map(paraStart) + 1)));
          return true;
        }

        // $formula$ → inline math atom (type closing $, cursor placed after)
        const m = /\$([^$\n]{1,200})$/.exec(textBefore);
        if (m) {
          const formula = m[1].trim();
          if (formula) {
            const start = cursor.pos - m[0].length;
            const inlineNode = typstSchema.nodes.math_inline.create({ formula });
            const tr = state.tr.replaceWith(start, cursor.pos, inlineNode);
            view.dispatch(tr.setSelection(TextSelection.create(tr.doc, tr.mapping.map(start) + inlineNode.nodeSize)));
            return true;
          }
        }

        // $$ → empty inline math atom, floating input auto-opens via NodeView constructor
        if (textBefore.endsWith('$')) {
          const start = cursor.pos - 1;
          const inlineNode = typstSchema.nodes.math_inline.create({ formula: '' });
          const tr = state.tr.replaceWith(start, cursor.pos, inlineNode);
          view.dispatch(tr.setSelection(TextSelection.create(tr.doc, tr.mapping.map(start) + inlineNode.nodeSize)));
          return true;
        }

        return false;
      },
    },
  });

  // ── Plugins & node views ─────────────────────────────────────────────────────
  export const typstPlugins = [
    mathFocusPlugin,
    mathKeyPlugin,
    inputRules({ rules: [
      textblockTypeInputRule(/^(={1,6})\s$/, typstSchema.nodes.heading, (m: RegExpMatchArray) => ({ level: m[1].length })),
      wrappingInputRule(/^\+\s$/, typstSchema.nodes.ordered_list),
    ]}),
  ];

  export const typstNodeViews = {
    math_inline: (node: PNode, ev: EditorView, gp: any) => new MathInlineNodeView(node, ev, gp),
    math_block:  (node: PNode, ev: EditorView, gp: any) => new MathNodeView(node, ev, gp),
  };

  // ── Serializer: doc → Typst ──────────────────────────────────────────────────
  function serializeInline(node: PNode): string {
    if (node.type.name === 'math_inline') return `$${node.attrs.formula}$`;
    if (node.isText) {
      let s = node.text ?? '';
      const marks = node.marks.map(m => m.type.name);
      if (marks.includes('strong')) s = `*${s}*`;
      if (marks.includes('em'))     s = `_${s}_`;
      if (marks.includes('code'))   s = '`' + s + '`';
      return s;
    }
    let out = ''; node.forEach(c => { out += serializeInline(c); }); return out;
  }

  function serializeBlock(node: PNode): string {
    switch (node.type.name) {
      case 'paragraph': {
        let s = ''; node.forEach(c => { s += serializeInline(c); }); return s + '\n';
      }
      case 'heading': {
        let s = ''; node.forEach(c => { s += serializeInline(c); });
        return '='.repeat(node.attrs.level) + ' ' + s + '\n';
      }
      case 'bullet_list': {
        let out = '';
        node.forEach(item => {
          let s = '';
          item.forEach(c => { if (c.type.name === 'paragraph') c.forEach(i => { s += serializeInline(i); }); });
          out += `- ${s}\n`;
        });
        return out;
      }
      case 'ordered_list': {
        let out = '';
        node.forEach(item => {
          let s = '';
          item.forEach(c => { if (c.type.name === 'paragraph') c.forEach(i => { s += serializeInline(i); }); });
          out += `+ ${s}\n`;
        });
        return out;
      }
      case 'blockquote': {
        let s = '';
        node.forEach(c => { if (c.type.name === 'paragraph') c.forEach(i => { s += serializeInline(i); }); });
        return `#quote[${s}]\n`;
      }
      case 'code_block':      return '```\n' + node.textContent + '\n```\n';
      case 'horizontal_rule': return '---\n';
      case 'math_block': {
        const f = node.textContent;
        return f.includes('\n') ? '$\n' + f + '\n$\n' : '$ ' + f + ' $\n';
      }
      case 'table': {
        const firstRow = node.firstChild;
        if (!firstRow) return '';
        const cols = firstRow.childCount;
        const cells: string[] = [];
        node.forEach(row => { row.forEach(cell => { cells.push('[' + cell.textContent.replace(/]/g, '\\]') + ']'); }); });
        return `#table(columns: ${cols},\n  ${cells.join(', ')},\n)\n`;
      }
      default: { let out = ''; node.forEach(c => { out += serializeBlock(c); }); return out; }
    }
  }

  export function serializeTypst(doc: PNode): string {
    let out = '';
    doc.forEach(block => { out += serializeBlock(block) + '\n'; });
    return out.replace(/\n{3,}/g, '\n\n').trimEnd() + '\n';
  }

  // ── Parser: Typst → doc ──────────────────────────────────────────────────────
  function mathInline(formula: string): PNode {
    return typstSchema.nodes.math_inline.create({ formula });
  }
  function mathBlock(formula: string): PNode {
    return typstSchema.nodes.math_block.create(null, formula ? [typstSchema.text(formula)] : []);
  }

  function parseInline(text: string): PNode[] {
    const nodes: PNode[] = [];
    let i = 0, plain = '';
    const flush = () => { if (plain) { nodes.push(typstSchema.text(plain)); plain = ''; } };
    while (i < text.length) {
      const rest = text.slice(i);
      const mathM = rest.match(/^\$([^$\n]+)\$/);
      if (mathM) { flush(); nodes.push(mathInline(mathM[1])); i += mathM[0].length; continue; }
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

  export function parseTypst(text: string): PNode {
    try {
      const blocks: PNode[] = [];
      const lines = text.split('\n');
      let i = 0;
      while (i < lines.length) {
        const line = lines[i];
        if (!line.trim()) { i++; continue; }

        const hm = line.match(/^(={1,6})\s+(.*)/);
        if (hm) { blocks.push(typstSchema.nodes.heading.create({ level: hm[1].length }, parseInline(hm[2]))); i++; continue; }

        if (/^- /.test(line)) {
          const items: PNode[] = [];
          while (i < lines.length && /^- /.test(lines[i]))
            items.push(typstSchema.nodes.list_item.create(null, [typstSchema.nodes.paragraph.create(null, parseInline(lines[i++].slice(2)))]));
          blocks.push(typstSchema.nodes.bullet_list.create(null, items)); continue;
        }

        if (/^\+ /.test(line)) {
          const items: PNode[] = [];
          while (i < lines.length && /^\+ /.test(lines[i]))
            items.push(typstSchema.nodes.list_item.create(null, [typstSchema.nodes.paragraph.create(null, parseInline(lines[i++].slice(2)))]));
          blocks.push(typstSchema.nodes.ordered_list.create(null, items)); continue;
        }

        if (line.trim() === '```') {
          i++;
          const codeLines: string[] = [];
          while (i < lines.length && lines[i].trim() !== '```') codeLines.push(lines[i++]);
          if (i < lines.length) i++;
          blocks.push(typstSchema.nodes.code_block.create(null, codeLines.length ? [typstSchema.text(codeLines.join('\n'))] : [])); continue;
        }

        // Single-line display math: $ formula $
        const sdm = line.match(/^\$ (.+) \$\s*$/);
        if (sdm) { blocks.push(mathBlock(sdm[1])); i++; continue; }

        // Multi-line display math: $ on its own line
        if (line.trim() === '$') {
          i++;
          const fLines: string[] = [];
          while (i < lines.length && lines[i].trim() !== '$') fLines.push(lines[i++]);
          if (i < lines.length) i++;
          blocks.push(mathBlock(fLines.join('\n'))); continue;
        }

        if (line.trim() === '---') { blocks.push(typstSchema.nodes.horizontal_rule.create()); i++; continue; }

        const qm = line.match(/^#quote\[(.+)\]$/);
        if (qm) { blocks.push(typstSchema.nodes.blockquote.create(null, [typstSchema.nodes.paragraph.create(null, parseInline(qm[1]))])); i++; continue; }

        const paraLines: string[] = [];
        while (i < lines.length) {
          const l = lines[i];
          if (!l.trim()) break;
          if (/^(={1,6}\s|```$|\$\s*$|\$ .+ \$\s*$|---$|#quote\[)/.test(l)) break;
          if (/^[+-] /.test(l) && paraLines.length > 0) break;
          paraLines.push(l); i++;
        }
        if (paraLines.length) blocks.push(typstSchema.nodes.paragraph.create(null, parseInline(paraLines.join('\n'))));
      }
      if (!blocks.length) blocks.push(typstSchema.nodes.paragraph.create());
      return typstSchema.nodes.doc.create(null, blocks);
    } catch { return typstSchema.topNodeType.createAndFill()!; }
  }
</script>

<script lang="ts">
  import ProseEditorBase, { type ToolbarItem } from './ProseEditorBase.svelte';
  import type { EditorView } from 'prosemirror-view';

  let { value = $bindable(''), placeholder = '', fillHeight = false }: {
    value: string; placeholder?: string; fillHeight?: boolean;
  } = $props();

  const mathToolbarItems: ToolbarItem[] = [
    {
      icon: '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4"><text x="2" y="10" font-size="9" font-family="serif" fill="currentColor" stroke="none">∑</text><line x1="9" y1="4" x2="13" y2="4"/><line x1="9" y1="7" x2="13" y2="7"/><line x1="9" y1="10" x2="13" y2="10"/></svg>',
      title: '插入行内公式 ($…$)',
      run(view: EditorView) {
        const { state, dispatch } = view;
        const node = typstSchema.nodes.math_inline.create({ formula: '' });
        dispatch(state.tr.replaceSelectionWith(node));
        // MathInlineNodeView constructor auto-opens floating input for empty nodes
      },
    },
    {
      icon: '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4"><rect x="1" y="3" width="12" height="8" rx="1"/><text x="4" y="10" font-size="7" font-family="serif" fill="currentColor" stroke="none">∫</text></svg>',
      title: '插入块级公式 ($$…$$)',
      run(view: EditorView) {
        const { state, dispatch } = view;
        const node = typstSchema.nodes.math_block.create(null, []);
        const from = state.selection.from;
        const tr = state.tr.replaceSelectionWith(node);
        dispatch(tr.setSelection(TextSelection.create(tr.doc, tr.mapping.map(from) + 1)));
      },
    },
  ];
</script>

<ProseEditorBase
  bind:value
  {placeholder}
  {fillHeight}
  schema={typstSchema}
  plugins={typstPlugins}
  nodeViews={typstNodeViews}
  serialize={serializeTypst}
  parse={parseTypst}
  headingPrefixes={['= ', '== ', '=== ']}
  toolbarItems={mathToolbarItems}
/>

<style>
  /* ── Math node wrapper ── */
  :global(.typst-math-inline-view) {
    display: inline-block;
    vertical-align: middle;
    position: relative;
    border-radius: 3px;
    cursor: text;
  }
  :global(.typst-math-block-view) {
    display: block;
    position: relative;
    border-radius: 4px;
    margin: 0.75em 0;
    text-align: center;
    cursor: text;
  }

  /* ── Source mode (cursor inside) ── */
  :global(.typst-math-inline-view.math-focused) {
    outline: 2px solid var(--accent, #4a7);
    outline-offset: 2px;
    background: rgba(95, 155, 101, 0.06);
  }
  :global(.typst-math-block-view.math-focused) {
    outline: 2px solid var(--accent, #4a7);
    outline-offset: 2px;
    background: rgba(95, 155, 101, 0.06);
    text-align: left;
  }
  /* $ delimiters shown as pseudo-elements, like heading prefixes */
  :global(.typst-math-inline-view.math-focused .math-source-text::before) { content: '$'; color: #888; user-select: none; }
  :global(.typst-math-inline-view.math-focused .math-source-text::after)  { content: '$'; color: #888; user-select: none; }
  :global(.typst-math-block-view.math-focused  .math-source-text::before) { content: '$ '; color: #888; user-select: none; }
  :global(.typst-math-block-view.math-focused  .math-source-text::after)  { content: ' $'; color: #888; user-select: none; }

  /* Source text styling */
  :global(.math-source-text) {
    font-family: var(--font-mono, monospace);
    font-size: 0.88em;
    color: #2a6b4a;
    white-space: pre-wrap;
  }

  /* ── Rendered mode ── */
  :global(.math-rendered) { display: inline-block; }
  :global(.typst-math-block-view .math-rendered) { display: block; }
  :global(.math-empty) {
    font-family: var(--font-mono, monospace);
    font-size: 0.88em;
    color: var(--text-hint, #aaa);
    background: rgba(42, 107, 74, 0.04);
    border-radius: 3px;
    padding: 0 4px;
    border: 1px dashed rgba(42, 107, 74, 0.3);
  }
  :global(.math-placeholder),
  :global(.math-fallback) {
    font-family: var(--font-mono, monospace);
    font-size: 0.88em;
    color: #2a6b4a;
    background: rgba(42, 107, 74, 0.07);
    border-radius: 3px;
    padding: 0 4px;
  }

  /* SVG sizing for MathML output */
  :global(.typst-math-inline-view svg),
  :global(.typst-math-block-view svg) { vertical-align: middle; max-width: 100%; height: auto; }

  /* Floating input for inline math editing */
  :global(.math-inline-input) {
    font-family: var(--font-mono, monospace) !important;
    font-size: 0.9em !important;
  }
</style>
