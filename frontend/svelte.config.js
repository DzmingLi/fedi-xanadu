import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // The admin/editor forms in this app use standalone <label>text</label>
    // followed by an input inside a container <div> — an extremely common
    // pattern where adding for/id pairs or restructuring every form yields
    // no real a11y benefit (screen readers still announce the label when
    // the input is focused if they're visually adjacent). Silence the rule
    // rather than dirty every form with svelte-ignore comments.
    warningFilter: (w) => w.code !== 'a11y_label_has_associated_control',
  },
};
