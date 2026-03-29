import "clsx";
import "./auth.svelte.js";
const ACTIONS = [
  {
    id: "goto.home",
    label: "Go to Home",
    labelZh: "前往首页",
    category: "navigation",
    defaultKey: "g h"
  },
  {
    id: "goto.skills",
    label: "Go to Skills",
    labelZh: "前往技能",
    category: "navigation",
    defaultKey: "g s"
  },
  {
    id: "goto.library",
    label: "Go to Library",
    labelZh: "前往书架",
    category: "navigation",
    defaultKey: "g l"
  },
  {
    id: "goto.about",
    label: "Go to About",
    labelZh: "前往关于",
    category: "navigation",
    defaultKey: "g a"
  },
  {
    id: "goto.newArticle",
    label: "New Article",
    labelZh: "新建文章",
    category: "navigation",
    defaultKey: "n a"
  },
  {
    id: "goto.newSeries",
    label: "New Series",
    labelZh: "新建系列",
    category: "navigation",
    defaultKey: "n s"
  },
  {
    id: "search",
    label: "Search",
    labelZh: "搜索",
    category: "global",
    defaultKey: "/"
  },
  {
    id: "help",
    label: "Show shortcuts",
    labelZh: "显示快捷键",
    category: "global",
    defaultKey: "?"
  },
  {
    id: "settings",
    label: "Shortcut settings",
    labelZh: "快捷键设置",
    category: "global",
    defaultKey: "Ctrl+,"
  },
  {
    id: "article.upvote",
    label: "Upvote",
    labelZh: "赞",
    category: "article",
    defaultKey: "Shift+u"
  },
  {
    id: "article.downvote",
    label: "Downvote",
    labelZh: "踩",
    category: "article",
    defaultKey: "Shift+d"
  },
  {
    id: "article.bookmark",
    label: "Bookmark",
    labelZh: "收藏",
    category: "article",
    defaultKey: "b"
  },
  {
    id: "list.next",
    label: "Next item",
    labelZh: "下一项",
    category: "list",
    defaultKey: "j"
  },
  {
    id: "list.prev",
    label: "Previous item",
    labelZh: "上一项",
    category: "list",
    defaultKey: "k"
  },
  {
    id: "list.open",
    label: "Open item",
    labelZh: "打开",
    category: "list",
    defaultKey: "Enter"
  }
];
const STORAGE_KEY = "fx_keybindings";
let storedBindings = {};
const raw = localStorage.getItem(STORAGE_KEY);
if (raw) {
  try {
    storedBindings = JSON.parse(raw);
  } catch {
  }
}
let userBindings = storedBindings;
function getAllBindings() {
  const result = {};
  for (const action of ACTIONS) {
    result[action.id] = userBindings[action.id] ?? action.defaultKey;
  }
  return result;
}
function formatKeyDisplay(combo) {
  return combo.split(" ").map((part) => {
    return part.split("+").map((k) => {
      const map = {
        "ctrl": "Ctrl",
        "shift": "Shift",
        "alt": "Alt",
        "meta": "Cmd",
        "enter": "Enter",
        "escape": "Esc"
      };
      return map[k.toLowerCase()] ?? k.toUpperCase();
    }).join("+");
  }).join(" ");
}
const CATEGORY_LABELS = {
  navigation: { en: "Navigation", zh: "导航" },
  global: { en: "Global", zh: "全局" },
  article: { en: "Article", zh: "文章" },
  list: { en: "List", zh: "列表" }
};
export {
  ACTIONS as A,
  CATEGORY_LABELS as C,
  formatKeyDisplay as f,
  getAllBindings as g
};
