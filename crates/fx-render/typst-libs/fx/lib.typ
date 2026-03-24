// Fedi-Xanadu standard library — auto-imported for all articles.

// --- Layout helpers ---
#let INDENT = 2em
#let indent = h(INDENT)
#let glue(indent: true, body) = body
#let section(title: "") = {}
#let section-counter = counter("section")
#let tablef(..args) = table(..args)

// --- Theorem environments ---
#let _thm-env(kind, cls) = {
  (body, name: none, id: none, breakable: true) => {
    html.elem("div", attrs: ("class": "thm-block thm-" + cls), {
      [*#kind#if name != none [ (#name)]*. ]
      body
    })
  }
}

#let definition = _thm-env("Definition", "defn")
#let theorem = _thm-env("Theorem", "thm")
#let lemma = _thm-env("Lemma", "thm")
#let corollary = _thm-env("Corollary", "thm")
#let proposition = _thm-env("Proposition", "thm")
#let proof = _thm-env("Proof", "proof")
#let remark = _thm-env("Remark", "remark")
#let example = _thm-env("Example", "example")
#let solution = _thm-env("Solution", "example")
