import "clsx";
import { N as NavBar } from "../../../chunks/NavBar.js";
function _layout($$renderer, $$props) {
  let { children } = $$props;
  $$renderer.push(`<div class="top-nav-wide svelte-1bk6358">`);
  NavBar($$renderer);
  $$renderer.push(`<!----></div> <div class="container-wide svelte-1bk6358">`);
  children($$renderer);
  $$renderer.push(`<!----></div>`);
}
export {
  _layout as default
};
