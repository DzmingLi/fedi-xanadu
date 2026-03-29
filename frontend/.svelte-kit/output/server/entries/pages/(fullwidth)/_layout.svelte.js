import "clsx";
import { N as NavBar } from "../../../chunks/NavBar.js";
function _layout($$renderer, $$props) {
  let { children } = $$props;
  $$renderer.push(`<div class="fullwidth-nav svelte-1wm97dc">`);
  NavBar($$renderer);
  $$renderer.push(`<!----></div> `);
  children($$renderer);
  $$renderer.push(`<!---->`);
}
export {
  _layout as default
};
