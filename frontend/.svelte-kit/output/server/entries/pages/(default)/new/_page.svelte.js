import { c as escape_html, i as attr, e as ensure_array_like, a as attr_class, b as stringify } from "../../../../chunks/index2.js";
import "@sveltejs/kit/internal";
import "../../../../chunks/exports.js";
import "../../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../../chunks/root.js";
import "../../../../chunks/state.svelte.js";
import "../../../../chunks/auth.svelte.js";
import { t, g as getLocale } from "../../../../chunks/index.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let savingDraft = false;
    let tags = [];
    let allArticles = [];
    let title = "";
    let description = "";
    let content = "";
    let contentFormat = "typst";
    let lang = getLocale();
    let license = "CC-BY-SA-4.0";
    let restricted = false;
    let translationOf = "";
    let category = "general";
    let selectedTags = [];
    let prereqs = [];
    let submitting = false;
    let uploadingImage = false;
    let loadingFile = false;
    let converting = false;
    async function handleFormatChange(newFormat) {
      contentFormat = newFormat;
      return;
    }
    let extraLangs = [];
    let newTagInput = "";
    let prereqTagId = "";
    let prereqType = "required";
    function getTagName(id) {
      return tags.find((t2) => t2.id === id)?.name ?? id;
    }
    $$renderer2.push(`<h1>${escape_html(t("newArticle.title"))}</h1> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="form-group svelte-5wzkq6"><label for="title">${escape_html(t("newArticle.titleLabel"))}</label> <input id="title"${attr("value", title)}${attr("placeholder", t("newArticle.titleLabel"))}/></div> <div class="form-group svelte-5wzkq6"><label for="description">${escape_html(t("newArticle.descLabel"))}</label> <input id="description"${attr("value", description)}${attr("placeholder", t("newArticle.descPlaceholder"))}/></div> <div class="form-row svelte-5wzkq6"><div class="form-group svelte-5wzkq6" style="flex:1"><label for="lang">${escape_html(t("newArticle.langLabel"))}</label> `);
    $$renderer2.select({ id: "lang", value: lang }, ($$renderer3) => {
      $$renderer3.option({ value: "zh" }, ($$renderer4) => {
        $$renderer4.push(`中文`);
      });
      $$renderer3.option({ value: "en" }, ($$renderer4) => {
        $$renderer4.push(`English`);
      });
      $$renderer3.option({ value: "ja" }, ($$renderer4) => {
        $$renderer4.push(`日本語`);
      });
      $$renderer3.option({ value: "ko" }, ($$renderer4) => {
        $$renderer4.push(`한국어`);
      });
      $$renderer3.option({ value: "fr" }, ($$renderer4) => {
        $$renderer4.push(`Français`);
      });
      $$renderer3.option({ value: "de" }, ($$renderer4) => {
        $$renderer4.push(`Deutsch`);
      });
    });
    $$renderer2.push(`</div> <div class="form-group svelte-5wzkq6" style="flex:1"><label for="license">${escape_html(t("newArticle.licenseLabel"))}</label> `);
    $$renderer2.select({ id: "license", value: license, disabled: restricted }, ($$renderer3) => {
      $$renderer3.option({ value: "CC-BY-NC-SA-4.0" }, ($$renderer4) => {
        $$renderer4.push(`CC BY-NC-SA 4.0`);
      });
      $$renderer3.option({ value: "CC-BY-SA-4.0" }, ($$renderer4) => {
        $$renderer4.push(`CC BY-SA 4.0`);
      });
      $$renderer3.option({ value: "CC-BY-4.0" }, ($$renderer4) => {
        $$renderer4.push(`CC BY 4.0`);
      });
      $$renderer3.option({ value: "CC-BY-NC-4.0" }, ($$renderer4) => {
        $$renderer4.push(`CC BY-NC 4.0`);
      });
      $$renderer3.option({ value: "CC-BY-NC-ND-4.0" }, ($$renderer4) => {
        $$renderer4.push(`CC BY-NC-ND 4.0`);
      });
      $$renderer3.option({ value: "CC0-1.0" }, ($$renderer4) => {
        $$renderer4.push(`CC0 (Public Domain)`);
      });
      $$renderer3.option({ value: "MIT" }, ($$renderer4) => {
        $$renderer4.push(`MIT`);
      });
      $$renderer3.option({ value: "Apache-2.0" }, ($$renderer4) => {
        $$renderer4.push(`Apache 2.0`);
      });
      $$renderer3.option({ value: "GFDL-1.3" }, ($$renderer4) => {
        $$renderer4.push(`GFDL 1.3`);
      });
      $$renderer3.option({ value: "All-Rights-Reserved" }, ($$renderer4) => {
        $$renderer4.push(`All Rights Reserved`);
      });
    });
    $$renderer2.push(` <label class="restricted-check svelte-5wzkq6"><input type="checkbox"${attr("checked", restricted, true)}/> ${escape_html(t("newArticle.restricted"))}</label></div> <div class="form-group svelte-5wzkq6" style="flex:1"><label for="category">${escape_html(t("newArticle.categoryLabel"))}</label> `);
    $$renderer2.select({ id: "category", value: category }, ($$renderer3) => {
      $$renderer3.option({ value: "general" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("category.general"))}`);
      });
      $$renderer3.option({ value: "lecture" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("category.lecture"))}`);
      });
      $$renderer3.option({ value: "paper" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("category.paper"))}`);
      });
      $$renderer3.option({ value: "review" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("category.review"))}`);
      });
    });
    $$renderer2.push(`</div> <div class="form-group svelte-5wzkq6" style="flex:2"><label for="translation-of">${escape_html(t("newArticle.translationOf"))}</label> `);
    $$renderer2.select({ id: "translation-of", value: translationOf }, ($$renderer3) => {
      $$renderer3.option({ value: "" }, ($$renderer4) => {
        $$renderer4.push(`${escape_html(t("newArticle.originalArticle"))}`);
      });
      $$renderer3.push(`<!--[-->`);
      const each_array = ensure_array_like(allArticles);
      for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
        let a = each_array[$$index];
        $$renderer3.option({ value: a.at_uri }, ($$renderer4) => {
          $$renderer4.push(`[${escape_html(a.lang)}] ${escape_html(a.title)}`);
        });
      }
      $$renderer3.push(`<!--]-->`);
    });
    $$renderer2.push(`</div></div> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="form-row svelte-5wzkq6"><div class="form-group svelte-5wzkq6" style="flex:1"><label for="format">${escape_html(t("newArticle.formatLabel"))}</label> `);
    $$renderer2.select(
      {
        id: "format",
        value: contentFormat,
        onchange: (e) => handleFormatChange(e.target.value),
        disabled: converting
      },
      ($$renderer3) => {
        $$renderer3.option({ value: "typst" }, ($$renderer4) => {
          $$renderer4.push(`Typst`);
        });
        $$renderer3.option({ value: "markdown" }, ($$renderer4) => {
          $$renderer4.push(`Markdown + KaTeX`);
        });
        $$renderer3.option({ value: "tex" }, ($$renderer4) => {
          $$renderer4.push(`LaTeX`);
        });
        $$renderer3.option({ value: "html" }, ($$renderer4) => {
          $$renderer4.push(`HTML`);
        });
      }
    );
    $$renderer2.push(` `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div></div> <div class="form-group svelte-5wzkq6"><div class="content-label-row svelte-5wzkq6"><label for="content" class="svelte-5wzkq6">${escape_html(t("newArticle.contentLabel"))} (${escape_html(contentFormat === "markdown" ? "Markdown" : contentFormat === "html" ? "HTML" : contentFormat === "tex" ? "LaTeX" : "Typst")})</label> <div class="upload-btns svelte-5wzkq6"><label${attr_class("upload-btn svelte-5wzkq6", void 0, { "disabled": loadingFile })}><input type="file" accept=".md,.markdown,.typ,.typst,.html,.htm,.tex,.latex" hidden=""/> ${escape_html(t("newArticle.uploadFile"))}</label> <label${attr_class("upload-btn svelte-5wzkq6", void 0, { "disabled": uploadingImage })}><input type="file" accept="image/*" hidden=""/> ${escape_html(t("newArticle.uploadImage"))}</label></div></div> <textarea id="content"${attr("placeholder", contentFormat === "markdown" ? "# My Article\n\nSome text with $x^2$ math" : contentFormat === "html" ? "<!DOCTYPE html>\n<html>\n<body>\n  <h1>My Article</h1>\n</body>\n</html>" : "= My Article")}>`);
    const $$body = escape_html(content);
    if ($$body) {
      $$renderer2.push(`${$$body}`);
    }
    $$renderer2.push(`</textarea></div> `);
    {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="form-group svelte-5wzkq6"><div class="lang-versions-header svelte-5wzkq6"><span class="form-label svelte-5wzkq6">${escape_html(t("newArticle.langVersions"))}</span> <button type="button" class="btn-add-lang svelte-5wzkq6">+ ${escape_html(t("newArticle.addLangVersion"))}</button></div> <!--[-->`);
      const each_array_2 = ensure_array_like(extraLangs);
      for (let idx = 0, $$length = each_array_2.length; idx < $$length; idx++) {
        let lv = each_array_2[idx];
        $$renderer2.push(`<div class="lang-version-block svelte-5wzkq6"><div class="lang-version-header svelte-5wzkq6">`);
        $$renderer2.select(
          { value: extraLangs[idx].lang, class: "" },
          ($$renderer3) => {
            $$renderer3.push(`<!--[-->`);
            const each_array_3 = ensure_array_like([
              ["zh", "中文"],
              ["en", "English"],
              ["ja", "日本語"],
              ["ko", "한국어"],
              ["fr", "Français"],
              ["de", "Deutsch"]
            ]);
            for (let $$index_2 = 0, $$length2 = each_array_3.length; $$index_2 < $$length2; $$index_2++) {
              let [code, name] = each_array_3[$$index_2];
              $$renderer3.option(
                {
                  value: code,
                  disabled: code === lang || extraLangs.some((l, i) => i !== idx && l.lang === code)
                },
                ($$renderer4) => {
                  $$renderer4.push(`${escape_html(name)}`);
                }
              );
            }
            $$renderer3.push(`<!--]-->`);
          },
          "svelte-5wzkq6"
        );
        $$renderer2.push(` `);
        $$renderer2.select(
          { value: extraLangs[idx].contentFormat, class: "" },
          ($$renderer3) => {
            $$renderer3.option({ value: "typst" }, ($$renderer4) => {
              $$renderer4.push(`Typst`);
            });
            $$renderer3.option({ value: "markdown" }, ($$renderer4) => {
              $$renderer4.push(`Markdown`);
            });
            $$renderer3.option({ value: "tex" }, ($$renderer4) => {
              $$renderer4.push(`LaTeX`);
            });
            $$renderer3.option({ value: "html" }, ($$renderer4) => {
              $$renderer4.push(`HTML`);
            });
          },
          "svelte-5wzkq6"
        );
        $$renderer2.push(` <label class="upload-btn svelte-5wzkq6"><input type="file" accept=".md,.markdown,.typ,.typst,.html,.htm,.tex,.latex" hidden=""/> ${escape_html(t("newArticle.uploadFile"))}</label> <button type="button" class="lang-remove svelte-5wzkq6">×</button></div> <textarea${attr("placeholder", t("newArticle.versionContent", lv.lang))} class="lang-textarea svelte-5wzkq6">`);
        const $$body_1 = escape_html(extraLangs[idx].content);
        if ($$body_1) {
          $$renderer2.push(`${$$body_1}`);
        }
        $$renderer2.push(`</textarea></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    }
    $$renderer2.push(`<!--]--> <div class="form-group svelte-5wzkq6"><label for="tag-input">${escape_html(t("newArticle.tagsLabel"))}</label> <div class="tag-input-row svelte-5wzkq6"><input id="tag-input" type="text"${attr("value", newTagInput)}${attr("placeholder", t("newArticle.tagInput"))} class="svelte-5wzkq6"/> <button type="button" class="tag-add-btn svelte-5wzkq6">${escape_html(t("common.add"))}</button> `);
    {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--></div> `);
    if (selectedTags.length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="selected-tags svelte-5wzkq6"><!--[-->`);
      const each_array_5 = ensure_array_like(selectedTags);
      for (let $$index_5 = 0, $$length = each_array_5.length; $$index_5 < $$length; $$index_5++) {
        let tagId = each_array_5[$$index_5];
        $$renderer2.push(`<span class="tag lit svelte-5wzkq6">${escape_html(getTagName(tagId))} <button type="button" class="tag-remove svelte-5wzkq6">×</button></span>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="tag-picker svelte-5wzkq6"><!--[-->`);
    const each_array_6 = ensure_array_like(tags.filter((t2) => !selectedTags.includes(t2.id)).slice(0, 20));
    for (let $$index_6 = 0, $$length = each_array_6.length; $$index_6 < $$length; $$index_6++) {
      let t2 = each_array_6[$$index_6];
      $$renderer2.push(`<button type="button" class="tag svelte-5wzkq6">${escape_html(t2.name)}</button>`);
    }
    $$renderer2.push(`<!--]--></div></div> <div class="form-group svelte-5wzkq6"><label for="prereq-select">${escape_html(t("newArticle.prereqsLabel"))}</label> <p class="form-hint svelte-5wzkq6">${escape_html(t("newArticle.prereqsHint"))}</p> `);
    if (prereqs.length > 0) {
      $$renderer2.push("<!--[0-->");
      $$renderer2.push(`<div class="prereq-list svelte-5wzkq6"><!--[-->`);
      const each_array_7 = ensure_array_like(prereqs);
      for (let $$index_7 = 0, $$length = each_array_7.length; $$index_7 < $$length; $$index_7++) {
        let p = each_array_7[$$index_7];
        $$renderer2.push(`<div class="prereq-item svelte-5wzkq6"><span${attr_class(`tag ${stringify(p.prereq_type)}`, "svelte-5wzkq6")}>${escape_html(getTagName(p.tag_id))}</span> <span class="prereq-type-label svelte-5wzkq6">${escape_html(p.prereq_type)}</span> <button class="prereq-remove svelte-5wzkq6"${attr("title", t("common.remove"))}>×</button></div>`);
      }
      $$renderer2.push(`<!--]--></div>`);
    } else {
      $$renderer2.push("<!--[-1-->");
    }
    $$renderer2.push(`<!--]--> <div class="prereq-add svelte-5wzkq6">`);
    $$renderer2.select(
      { id: "prereq-select", value: prereqTagId, class: "" },
      ($$renderer3) => {
        $$renderer3.option({ value: "" }, ($$renderer4) => {
          $$renderer4.push(`${escape_html(t("newArticle.selectTag"))}`);
        });
        $$renderer3.push(`<!--[-->`);
        const each_array_8 = ensure_array_like(tags.filter((t2) => !prereqs.some((p) => p.tag_id === t2.id)));
        for (let $$index_8 = 0, $$length = each_array_8.length; $$index_8 < $$length; $$index_8++) {
          let t2 = each_array_8[$$index_8];
          $$renderer3.option({ value: t2.id }, ($$renderer4) => {
            $$renderer4.push(`${escape_html(t2.name)}`);
          });
        }
        $$renderer3.push(`<!--]-->`);
      },
      "svelte-5wzkq6"
    );
    $$renderer2.push(` `);
    $$renderer2.select(
      { value: prereqType, class: "" },
      ($$renderer3) => {
        $$renderer3.option({ value: "required" }, ($$renderer4) => {
          $$renderer4.push(`${escape_html(t("newArticle.required"))}`);
        });
        $$renderer3.option({ value: "recommended" }, ($$renderer4) => {
          $$renderer4.push(`${escape_html(t("newArticle.recommended"))}`);
        });
        $$renderer3.option({ value: "suggested" }, ($$renderer4) => {
          $$renderer4.push(`${escape_html(t("newArticle.suggested"))}`);
        });
      },
      "svelte-5wzkq6"
    );
    $$renderer2.push(` <button class="prereq-add-btn svelte-5wzkq6"${attr("disabled", !prereqTagId, true)}>${escape_html(t("newArticle.addPrereq"))}</button></div></div> `);
    {
      $$renderer2.push("<!--[-1-->");
      $$renderer2.push(`<div class="submit-row svelte-5wzkq6">`);
      {
        $$renderer2.push("<!--[0-->");
        $$renderer2.push(`<button class="btn btn-draft svelte-5wzkq6"${attr("disabled", savingDraft, true)}>${escape_html(t("newArticle.saveDraft"))}</button>`);
      }
      $$renderer2.push(`<!--]--> `);
      {
        $$renderer2.push("<!--[-1-->");
      }
      $$renderer2.push(`<!--]--> <button class="btn btn-primary"${attr("disabled", submitting, true)}>${escape_html(t("newArticle.publish"))}</button></div>`);
    }
    $$renderer2.push(`<!--]-->`);
  });
}
export {
  _page as default
};
