import "clsx";
import { N as NavBar } from "../../../chunks/NavBar.js";
function _layout($$renderer, $$props) {
  let { children } = $$props;
  $$renderer.push(`<div class="top-nav svelte-1iqk2by">`);
  NavBar($$renderer);
  $$renderer.push(`<!----></div> <div class="container">`);
  children($$renderer);
  $$renderer.push(`<!----></div>`);
}
export {
  _layout as default
};
