<script lang="ts">
  import { Schema } from 'prosemirror-model';
  import { schema as basicSchema } from 'prosemirror-schema-basic';
  import { addListNodes } from 'prosemirror-schema-list';
  import { defaultMarkdownParser, defaultMarkdownSerializer, MarkdownParser, MarkdownSerializer } from 'prosemirror-markdown';
  import { tableNodes } from 'prosemirror-tables';
  import ProseEditorBase from './ProseEditorBase.svelte';

  let { value = $bindable(''), placeholder = '', fillHeight = false }: {
    value: string; placeholder?: string; fillHeight?: boolean;
  } = $props();

  // ── Schema ──────────────────────────────────────────────────────────────
  const baseNodes = addListNodes(basicSchema.spec.nodes, 'paragraph block*', 'block');
  const mdSchema = new Schema({
    nodes: (baseNodes as any).append(tableNodes({ tableGroup: 'block', cellContent: 'block+', cellAttributes: {} })),
    marks: basicSchema.spec.marks,
  });

  // ── Parser ───────────────────────────────────────────────────────────────
  const mdParser = new MarkdownParser(mdSchema, defaultMarkdownParser.tokenizer, {
    ...defaultMarkdownParser.tokens,
  });

  function parse(text: string) {
    try { return mdParser.parse(text) ?? mdSchema.topNodeType.createAndFill()!; }
    catch { return mdSchema.topNodeType.createAndFill()!; }
  }

  // ── Serializer (extended to handle table nodes) ──────────────────────────
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

  function serialize(doc: any): string {
    return mdSerializer.serialize(doc);
  }
</script>

<ProseEditorBase
  bind:value
  {placeholder}
  {fillHeight}
  schema={mdSchema}
  {serialize}
  {parse}
  headingPrefixes={['# ', '## ', '### ']}
/>
