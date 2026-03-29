import "clsx";
import { N as NavBar } from "../../../chunks/NavBar.js";
function _layout($$renderer, $$props) {
  let { children } = $$props;
  $$renderer.push(`<div class="top-nav svelte-1mwv8db">`);
  NavBar($$renderer);
  $$renderer.push(`<!----></div> <div class="container article-view">`);
  children($$renderer);
  $$renderer.push(`<!----></div>`);
}
export {
  _layout as default
};
