document.getElementById("new-article-form").onsubmit = function(e) {
    e.preventDefault();
    var tagSelect = document.getElementById("tags");
    var tags = [];
    for (var i = 0; i < tagSelect.selectedOptions.length; i++) {
        tags.push(tagSelect.selectedOptions[i].value);
    }
    var body = {
        title: document.getElementById("title").value,
        content: document.getElementById("content").value,
        content_format: "typst",
        tags: tags,
        prereqs: []
    };
    fetch("/api/articles", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body)
    }).then(function(res) {
        if (res.ok) {
            return res.json().then(function(article) {
                window.location.href = "/article?uri=" + encodeURIComponent(article.at_uri);
            });
        } else {
            return res.text().then(function(t) { alert("Error: " + t); });
        }
    });
};
