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
  // Math nodes store the formula as text content (like heading stores heading
  // text). This lets the cursor enter naturally, exactly like headings.
  const baseNodes = addListNodes(basicSchema.spec.nodes, 'paragraph block*', 'block');
  export const typstSchema = new Schema({
    nodes: (baseNodes as any)
      .append(tableNodes({ tableGroup: 'block', cellContent: 'block+', cellAttributes: {} }))
      .append({
        math_inline: {
          group: 'inline', inline: true,
          content: 'text*',
          marks: '',
          parseDOM: [{ tag: 'span.typst-math-inline' }],
          toDOM: () => ['span', { class: 'typst-math-inline' }, 0],
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
      console.warn('[math] render API returned', res.status, 'for formula:', formula);
    } catch (err) {
      console.warn('[math] render API fetch failed:', err);
    }
    return display
      ? `<span class="math-fallback">$ ${formula} $</span>`
      : `<span class="math-fallback">$${formula}$</span>`;
  }

  // ── MathNodeView ─────────────────────────────────────────────────────────────
  // Mirrors the heading NodeView pattern:
  //   cursor inside  → source mode: formula text visible + $ delimiters via CSS
  //   cursor outside → rendered mode: MathML shown, formula text collapsed
  //
  // The mathFocusPlugin below adds a `mathFocused` node decoration when the
  // cursor is inside a math node; this NodeView reacts to that decoration.
  export class MathNodeView implements NodeView {
    dom: HTMLElement;
    contentDOM: HTMLElement;          // ProseMirror manages formula text here
    private _display: boolean;
    private _renderEl: HTMLElement;   // shows compiled MathML
    private _focused = false;
    private _lastFormula = '';

    constructor(node: PNode, _view: EditorView, _getPos: any) {
      this._display = node.type.name === 'math_block';

      this.dom = document.createElement(this._display ? 'div' : 'span');
      this.dom.className = this._display ? 'typst-math-block-view' : 'typst-math-inline-view';

      // contentDOM: editable formula text (always in DOM, collapsed when rendered)
      this.contentDOM = document.createElement(this._display ? 'div' : 'span');
      this.contentDOM.className = 'math-source-text';

      // renderEl: MathML output (visible when cursor is outside)
      this._renderEl = document.createElement(this._display ? 'div' : 'span');
      this._renderEl.className = 'math-rendered';

      this.dom.appendChild(this.contentDOM);
      this.dom.appendChild(this._renderEl);

      // Start in rendered mode
      this._applyRendered(node.textContent);
    }

    update(node: PNode, decorations: readonly any[]) {
      const expectedType = this._display ? typstSchema.nodes.math_block : typstSchema.nodes.math_inline;
      if (node.type !== expectedType) return false;

      const focused = decorations.some((d: any) => d.spec?.mathFocused);
      const formula = node.textContent;

      if (focused !== this._focused) {
        this._focused = focused;
        focused ? this._applySource() : this._applyRendered(formula);
      } else if (!focused && formula !== this._lastFormula) {
        // Programmatic formula change while not focused
        this._applyRendered(formula);
      }
      return true;
    }

    // Show the formula text for editing (cursor is inside).
    private _applySource() {
      this.dom.classList.add('math-focused');
      // Restore contentDOM to normal flow
      this.contentDOM.style.cssText = '';
      this._renderEl.style.display = 'none';
    }

    // Show the compiled MathML (cursor is outside).
    // contentDOM is collapsed to zero size (not display:none, so ProseMirror can
    // still position the cursor there if the user arrows into the node).
    private _applyRendered(formula: string) {
      this._lastFormula = formula;
      this.dom.classList.remove('math-focused');
      // Collapse contentDOM — keeps it in DOM for ProseMirror but takes no space
      this.contentDOM.style.cssText = this._display
        ? 'display:block;height:0;overflow:hidden'
        : 'display:inline-block;width:0;height:0;overflow:hidden;vertical-align:middle';
      this._renderEl.style.display = '';
      if (!formula) {
        this._renderEl.innerHTML = this._display
          ? `<span class="math-empty">$ $</span>`
          : `<span class="math-empty">$ $</span>`;
        return;
      }
      // Show placeholder while the async compile runs
      this._renderEl.innerHTML = this._display
        ? `<span class="math-placeholder">$ ${formula} $</span>`
        : `<span class="math-placeholder">$${formula}$</span>`;
      fetchMathHtml(formula, this._display).then(html => {
        if (!this._focused && this._lastFormula === formula) this._renderEl.innerHTML = html;
      });
    }

    // Ignore mutations in _renderEl (we update it ourselves).
    // Let ProseMirror handle contentDOM mutations normally.
    ignoreMutation(mut: MutationRecord | { type: 'selection'; target: Element }) {
      const target = (mut as MutationRecord).target;
      if (!target) return false;
      return !this.contentDOM.contains(target as Node) && target !== this.contentDOM;
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
  // $formula$   → math_inline, cursor placed after the node
  // $$formula$$ → math_block,  cursor placed after the block
  // $ $         → empty math_block, cursor placed inside (to type formula)
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

        // "$" → empty inline math node, cursor inside (user typed $$ with nothing between)
        if (textBefore === '$') {
          const start = cursor.pos - 1;
          const inlineNode = typstSchema.nodes.math_inline.create(null, []);
          const tr = state.tr.replaceWith(start, cursor.pos, inlineNode);
          view.dispatch(tr.setSelection(TextSelection.create(tr.doc, tr.mapping.map(start) + 1)));
          return true;
        }

        // $formula$ → inline math, cursor after
        const m = /\$([^$\n]{1,200})$/.exec(textBefore);
        if (!m) return false;
        const formula = m[1].trim();
        if (!formula) return false;

        const start = cursor.pos - m[0].length;
        const inlineNode = typstSchema.nodes.math_inline.create(null, [typstSchema.text(formula)]);
        const tr = state.tr.replaceWith(start, cursor.pos, inlineNode);
        // Cursor after the inline node
        view.dispatch(tr.setSelection(TextSelection.create(tr.doc, tr.mapping.map(start) + inlineNode.nodeSize)));
        return true;
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
    math_inline: (node: PNode, ev: EditorView, gp: any) => new MathNodeView(node, ev, gp),
    math_block:  (node: PNode, ev: EditorView, gp: any) => new MathNodeView(node, ev, gp),
  };

  // ── Serializer: doc → Typst ──────────────────────────────────────────────────
  function serializeInline(node: PNode): string {
    if (node.type.name === 'math_inline') return `$${node.textContent}$`;
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
    return typstSchema.nodes.math_inline.create(null, formula ? [typstSchema.text(formula)] : []);
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
        const node = typstSchema.nodes.math_inline.create(null, []);
        const tr = state.tr.replaceSelectionWith(node);
        // Cursor inside the new node
        const nodeStart = tr.selection.from - node.nodeSize;
        dispatch(tr.setSelection(TextSelection.create(tr.doc, nodeStart + 1)));
      },
    },
    {
      icon: '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.4"><rect x="1" y="3" width="12" height="8" rx="1"/><text x="4" y="10" font-size="7" font-family="serif" fill="currentColor" stroke="none">∫</text></svg>',
      title: '插入块级公式 ($$…$$)',
      run(view: EditorView) {
        const { state, dispatch } = view;
        const node = typstSchema.nodes.math_block.create(null, []);
        const tr = state.tr.replaceSelectionWith(node);
        const nodeStart = tr.selection.from - node.nodeSize;
        dispatch(tr.setSelection(TextSelection.create(tr.doc, nodeStart + 1)));
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
</style>
