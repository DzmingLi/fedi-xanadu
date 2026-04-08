<script module lang="ts">
  import { Schema, type Node as PNode } from 'prosemirror-model';
  import { schema as basicSchema } from 'prosemirror-schema-basic';
  import { addListNodes } from 'prosemirror-schema-list';
  import { tableNodes } from 'prosemirror-tables';
  import { inputRules, InputRule, textblockTypeInputRule, wrappingInputRule } from 'prosemirror-inputrules';
  import { Plugin, NodeSelection } from 'prosemirror-state';
  import { type NodeView } from 'prosemirror-view';
  import type { EditorView } from 'prosemirror-view';

  // ── Module-level singletons ──────────────────────────────────────────────

  const baseNodes = addListNodes(basicSchema.spec.nodes, 'paragraph block*', 'block');
  export const typstSchema = new Schema({
    nodes: (baseNodes as any)
      .append(tableNodes({ tableGroup: 'block', cellContent: 'block+', cellAttributes: {} }))
      .append({
        math_inline: {
          group: 'inline', inline: true, atom: true,
          attrs: { formula: { default: '' } },
          parseDOM: [{ tag: 'span[data-math]', getAttrs: (d: any) => ({ formula: d.dataset.math }) }],
          toDOM: (n: PNode) => ['span', { 'data-math': n.attrs.formula, class: 'typst-math-inline' }],
        },
        math_block: {
          group: 'block', atom: true,
          attrs: { formula: { default: '' } },
          parseDOM: [{ tag: 'div[data-math-block]', getAttrs: (d: any) => ({ formula: d.dataset.mathBlock }) }],
          toDOM: (n: PNode) => ['div', { 'data-math-block': n.attrs.formula, class: 'typst-math-block' }],
        },
      }),
    marks: basicSchema.spec.marks,
  });

  // ── Math rendering cache (shared across all TypstEditor instances) ────────
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
    return display
      ? `<span class="math-fallback">$ ${formula} $</span>`
      : `<span class="math-fallback">$${formula}$</span>`;
  }

  // ── MathNodeView — no closure over component scope ────────────────────────
  // Receives schema via typstSchema (module-level) and fetchMathHtml (module-level).
  export class MathNodeView implements NodeView {
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
      const f = this._formula, d = this._display, dom = this.dom;
      dom.innerHTML = d
        ? `<span class="math-source">$ ${f} $</span>`
        : `<span class="math-source">$${f}$</span>`;
      fetchMathHtml(f, d).then(html => {
        if (!this._editing && this._formula === f) dom.innerHTML = html;
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
        // Show formula with $$ delimiters (Markdown/LaTeX convention)
        const hasNewlines = this._formula.includes('\n');
        const displayed = hasNewlines ? `$$\n${this._formula}\n$$` : `$$${this._formula}$$`;
        ta.value = displayed;
        ta.rows = Math.max(3, displayed.split('\n').length + 1);
        ta.addEventListener('keydown', e => {
          if (e.key === 'Escape' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) {
            e.preventDefault(); e.stopPropagation(); this._commitEdit(ta.value);
          }
          e.stopPropagation();
        });
        ta.addEventListener('blur', () => this._commitEdit(ta.value));
        this.dom.appendChild(ta);
        this._editorEl = ta;
        // Select just the formula content between the $$ markers
        const selStart = hasNewlines ? 3 : 2;
        const selEnd = displayed.length - (hasNewlines ? 3 : 2);
        setTimeout(() => { ta.focus(); ta.setSelectionRange(selStart, Math.max(selStart, selEnd)); }, 0);
      } else {
        const inp = document.createElement('input');
        inp.type = 'text';
        inp.className = 'math-edit-input';
        inp.value = `$${this._formula}$`;
        inp.addEventListener('keydown', e => {
          if (e.key === 'Enter' || e.key === 'Escape') {
            e.preventDefault(); e.stopPropagation(); this._commitEdit(inp.value);
          }
          e.stopPropagation();
        });
        inp.addEventListener('blur', () => this._commitEdit(inp.value));
        this.dom.appendChild(inp);
        this._editorEl = inp;
        // Select just the formula content (not the $ signs)
        setTimeout(() => { inp.focus(); inp.setSelectionRange(1, Math.max(1, inp.value.length - 1)); }, 0);
      }
    }

    private _commitEdit(rawValue: string) {
      if (!this._editing) return;
      this._editing = false;
      this._editorEl = null;
      this.dom.classList.remove('editing');

      const v = rawValue.trim();
      let wantDisplay = this._display;
      let formula: string;

      // Determine intended type from delimiters: $$ = display, $ = inline.
      // Check $$ before $ to avoid treating $$…$$ as an inline $…$.
      if (v.startsWith('$$') && v.endsWith('$$') && v.length > 4) {
        wantDisplay = true;
        formula = v.slice(2, -2).trim();
      } else if (v.startsWith('$') && v.endsWith('$') && v.length > 2 && !v.startsWith('$$')) {
        wantDisplay = false;
        formula = v.slice(1, -1).trim();
      } else {
        // No recognizable delimiters: treat as raw formula, keep current type.
        formula = v;
      }

      if (!formula) { this._renderMath(); return; }

      const pos = typeof this._getPos === 'function' ? this._getPos() : undefined;
      if (!this._view || pos === undefined) {
        this._formula = formula; this._renderMath(); return;
      }

      const state = this._view.state;

      if (wantDisplay === this._display) {
        // Same type: update formula if changed.
        if (formula !== this._formula) {
          const nodeType = this._display ? typstSchema.nodes.math_block : typstSchema.nodes.math_inline;
          this._view.dispatch(state.tr.setNodeMarkup(pos, nodeType, { formula }));
        } else {
          this._formula = formula; this._renderMath();
        }
        return;
      }

      // Type conversion between inline ↔ display.
      const currentNode = state.doc.nodeAt(pos);
      if (!currentNode) { this._formula = formula; this._renderMath(); return; }

      if (!this._display && wantDisplay) {
        // inline → display: only convert when the inline math is the sole
        // content of its paragraph (otherwise keep inline, just update formula).
        const rPos = state.doc.resolve(pos);
        const parent = rPos.parent;
        if (parent.type.name === 'paragraph' && parent.childCount === 1) {
          const paraStart = rPos.before(rPos.depth);
          const block = typstSchema.nodes.math_block.create({ formula });
          this._view.dispatch(state.tr.replaceWith(paraStart, paraStart + parent.nodeSize, block));
        } else {
          this._view.dispatch(state.tr.setNodeMarkup(pos, typstSchema.nodes.math_inline, { formula }));
        }
      } else {
        // display → inline: replace block with a paragraph containing inline math.
        const inlineNode = typstSchema.nodes.math_inline.create({ formula });
        const para = typstSchema.nodes.paragraph.create(null, [inlineNode]);
        this._view.dispatch(state.tr.replaceWith(pos, pos + currentNode.nodeSize, para));
      }
    }

    update(node: PNode) {
      const expectedType = this._display ? typstSchema.nodes.math_block : typstSchema.nodes.math_inline;
      if (node.type !== expectedType) return false;
      const f = node.attrs.formula;
      if (f !== this._formula) { this._formula = f; if (!this._editing) this._renderMath(); }
      return true;
    }
    selectNode()   { this.dom.classList.add('selected'); this._startEditing(); }
    deselectNode() { this.dom.classList.remove('selected'); if (this._editing && this._editorEl) this._commitEdit(this._editorEl.value); }
    stopEvent(e: Event) { return this._editing && (e.target === this._editorEl || this.dom.contains(e.target as Node)); }
    ignoreMutation() { return true; }
    destroy() { this._view = null; }
  }

  // ── Math inline creation via keydown ─────────────────────────────────────
  // Intercepts the closing $ key BEFORE insertion so we never dispatch a
  // replaceWith inside an in-progress input event (which freezes the editor
  // and also doesn't fire at all when a Chinese IME is active).
  const mathKeyPlugin = new Plugin({
    props: {
      handleKeyDown(view, e) {
        // Only handle a plain $ key (not during IME composition).
        if (e.key !== '$' || e.isComposing) return false;
        const sel = view.state.selection;
        const cursor = (sel as any).$cursor;
        if (!cursor || cursor.parent.type.name !== 'paragraph') return false;
        // Text in the current paragraph up to the cursor.
        const textBefore = cursor.parent.textBetween(0, cursor.parentOffset, null, '\ufffc');
        // $$formula$$ → display math block (triggered on the second closing $).
        // textBefore will be "$$formula$" when the user presses the final $.
        const ddMatch = /^\$\$([^$\n]+)\$$/.exec(textBefore);
        if (ddMatch) {
          const formula = ddMatch[1].trim();
          if (formula) {
            const nodePos = cursor.before();
            const nodeEnd = nodePos + cursor.parent.nodeSize;
            const block = typstSchema.nodes.math_block.create({ formula });
            const tr = view.state.tr.replaceWith(nodePos, nodeEnd, block);
            view.dispatch(tr.setSelection(NodeSelection.create(tr.doc, nodePos)));
            return true;
          }
        }
        // "$ " (dollar + space/newline before closing $) → empty display math block.
        if (/\$\s+$/.test(textBefore)) {
          const nodePos = cursor.before();
          const nodeEnd = nodePos + cursor.parent.nodeSize;
          const block = typstSchema.nodes.math_block.create({ formula: '' });
          const tr = view.state.tr.replaceWith(nodePos, nodeEnd, block);
          view.dispatch(tr.setSelection(NodeSelection.create(tr.doc, nodePos)));
          return true;
        }
        // If the paragraph starts with $$ (user building $$formula$$),
        // let this $ be inserted literally — don't trigger inline math on
        // the first closing $.
        if (/^\$\$/.test(textBefore)) return false;
        // Match the last unmatched $…: everything after the last $ sign.
        const m = /\$([^$\n]{1,200})$/.exec(textBefore);
        if (!m) return false;
        const formula = m[1].trim();
        if (!formula) return false;
        // Replace "$formula" (the opening $ + content, without closing $)
        // with a math_inline node. The closing $ is consumed by returning true.
        const start = cursor.pos - m[0].length;
        view.dispatch(
          view.state.tr.replaceWith(start, cursor.pos, typstSchema.nodes.math_inline.create({ formula }))
        );
        return true;
      },
    },
  });

  // ── Input rules ──────────────────────────────────────────────────────────
  // The $ InputRule below is a fallback for paste / non-keydown paths.
  // For live typing the mathKeyPlugin above takes precedence (and returns true
  // before the $ is inserted, so the InputRule never sees it).
  export const typstPlugins = [
    mathKeyPlugin,
    inputRules({ rules: [
      textblockTypeInputRule(/^(={1,6})\s$/, typstSchema.nodes.heading, (m: RegExpMatchArray) => ({ level: m[1].length })),
      wrappingInputRule(/^\+\s$/, typstSchema.nodes.ordered_list),
      new InputRule(/\$([^$\n]{1,200})\$$/, (state, match, start, end) => {
        const formula = match[1].trim();
        if (!formula) return null;
        return state.tr.replaceWith(start, end, typstSchema.nodes.math_inline.create({ formula }));
      }),
    ]}),
  ];

  export const typstNodeViews = {
    math_inline: (node: PNode, ev: EditorView, gp: any) => new MathNodeView(node, ev, gp),
    math_block:  (node: PNode, ev: EditorView, gp: any) => new MathNodeView(node, ev, gp),
  };

  // ── Serializer: doc → Typst ──────────────────────────────────────────────
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
      case 'math_block':      return '$\n' + node.attrs.formula + '\n$\n';
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

  // ── Parser: Typst → doc ──────────────────────────────────────────────────
  function parseInline(text: string): PNode[] {
    const nodes: PNode[] = [];
    let i = 0, plain = '';
    const flush = () => { if (plain) { nodes.push(typstSchema.text(plain)); plain = ''; } };
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
          while (i < lines.length && /^- /.test(lines[i])) {
            items.push(typstSchema.nodes.list_item.create(null, [typstSchema.nodes.paragraph.create(null, parseInline(lines[i].slice(2)))]));
            i++;
          }
          blocks.push(typstSchema.nodes.bullet_list.create(null, items)); continue;
        }
        if (/^\+ /.test(line)) {
          const items: PNode[] = [];
          while (i < lines.length && /^\+ /.test(lines[i])) {
            items.push(typstSchema.nodes.list_item.create(null, [typstSchema.nodes.paragraph.create(null, parseInline(lines[i].slice(2)))]));
            i++;
          }
          blocks.push(typstSchema.nodes.ordered_list.create(null, items)); continue;
        }
        if (line.trim() === '```') {
          i++;
          const codeLines: string[] = [];
          while (i < lines.length && lines[i].trim() !== '```') codeLines.push(lines[i++]);
          if (i < lines.length) i++;
          blocks.push(typstSchema.nodes.code_block.create(null, codeLines.length ? [typstSchema.text(codeLines.join('\n'))] : [])); continue;
        }
        if (line.trim() === '$') {
          i++;
          const fLines: string[] = [];
          while (i < lines.length && lines[i].trim() !== '$') fLines.push(lines[i++]);
          if (i < lines.length) i++;
          blocks.push(typstSchema.nodes.math_block.create({ formula: fLines.join('\n') })); continue;
        }
        if (line.trim() === '---') { blocks.push(typstSchema.nodes.horizontal_rule.create()); i++; continue; }
        const qm = line.match(/^#quote\[(.+)\]$/);
        if (qm) { blocks.push(typstSchema.nodes.blockquote.create(null, [typstSchema.nodes.paragraph.create(null, parseInline(qm[1]))])); i++; continue; }
        const paraLines: string[] = [];
        while (i < lines.length) {
          const l = lines[i];
          if (!l.trim()) break;
          if (/^(={1,6}\s|```$|\$\s*$|---$|#quote\[)/.test(l)) break;
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

  // ── Math insert toolbar items ────────────────────────────────────────────
  const mathToolbarItems: ToolbarItem[] = [
    {
      icon: '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4"><text x="2" y="10" font-size="9" font-family="serif" fill="currentColor" stroke="none">∑</text><line x1="9" y1="4" x2="13" y2="4"/><line x1="9" y1="7" x2="13" y2="7"/><line x1="9" y1="10" x2="13" y2="10"/></svg>',
      title: '插入行内公式 ($…$)',
      run(view: EditorView) {
        const { state, dispatch } = view;
        const node = typstSchema.nodes.math_inline.create({ formula: '' });
        dispatch(state.tr.replaceSelectionWith(node));
      },
    },
    {
      icon: '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4"><rect x="1" y="3" width="12" height="8" rx="1"/><text x="4" y="10" font-size="7" font-family="serif" fill="currentColor" stroke="none">∫</text></svg>',
      title: '插入块级公式 ($$…$$)',
      run(view: EditorView) {
        const { state, dispatch } = view;
        const node = typstSchema.nodes.math_block.create({ formula: '' });
        dispatch(state.tr.replaceSelectionWith(node));
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
  /* ── Typst math nodes ── */
  :global(.typst-math-inline-view) { display: inline-block; vertical-align: middle; cursor: pointer; border-radius: 3px; padding: 0 2px; }
  :global(.typst-math-block-view)  { display: block; border-radius: 4px; margin: 0.75em 0; text-align: center; cursor: pointer; }
  :global(.typst-math-inline-view.selected),
  :global(.typst-math-block-view.selected) { outline: 2px solid var(--accent, #4a7); outline-offset: 2px; background: rgba(95, 155, 101, 0.06); }
  :global(.math-source),
  :global(.math-fallback) { font-family: var(--font-mono, monospace); font-size: 0.88em; color: #2a6b4a; background: rgba(42, 107, 74, 0.07); border-radius: 3px; padding: 0 4px; }
  :global(.typst-math-inline-view.editing) { outline: 2px solid var(--accent, #4a7); outline-offset: 2px; background: rgba(95, 155, 101, 0.06); display: inline-flex; align-items: center; }
  :global(.typst-math-block-view.editing)  { outline: 2px solid var(--accent, #4a7); outline-offset: 2px; background: rgba(95, 155, 101, 0.06); display: block; }
  :global(.math-edit-input) { font-family: var(--font-mono, monospace); font-size: 0.9em; border: none; outline: none; background: transparent; color: #2a6b4a; padding: 2px 4px; min-width: 6ch; width: auto; resize: none; line-height: 1.4; }
  :global(textarea.math-edit-input) { display: block; width: 100%; min-height: 2em; }
  :global(.typst-math-inline-view svg),
  :global(.typst-math-block-view svg) { vertical-align: middle; max-width: 100%; height: auto; }
</style>
