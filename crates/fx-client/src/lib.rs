//! Typed HTTP client for the NightBoat API.
//!
//! Used by fx-cli and integration tests instead of raw reqwest calls.

pub mod api;
mod error;

pub use error::{ClientError, ClientResult};

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

/// Typed HTTP client for NightBoat.
#[derive(Debug, Clone)]
pub struct FxClient {
    base_url: String,
    token: Option<String>,
    admin_secret: Option<String>,
    http: reqwest::Client,
}

impl FxClient {
    /// Create a new client pointed at the given base URL (e.g. `http://localhost:3000/api`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: None,
            admin_secret: None,
            http: reqwest::Client::new(),
        }
    }

    /// Set the auth token (Bearer token from login).
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the admin secret for admin endpoints.
    pub fn with_admin_secret(mut self, secret: impl Into<String>) -> Self {
        self.admin_secret = Some(secret.into());
        self
    }

    /// Replace the auth token at runtime (e.g. after login).
    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    /// Clear the auth token.
    pub fn clear_token(&mut self) {
        self.token = None;
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // ---- Internal HTTP helpers ----

    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(ref token) = self.token {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        if let Some(ref secret) = self.admin_secret {
            if let Ok(val) = HeaderValue::from_str(secret) {
                headers.insert("x-admin-secret", val);
            }
        }
        headers
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let resp = self
            .http
            .get(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn get_with_query<T: serde::de::DeserializeOwned, Q: serde::Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> ClientResult<T> {
        let resp = self
            .http
            .get(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .query(query)
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn post<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let resp = self
            .http
            .post(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn post_empty(&self, path: &str) -> ClientResult<()> {
        let resp = self
            .http
            .post(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .send()
            .await?;
        Self::handle_empty(resp).await
    }

    async fn put<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let resp = self
            .http
            .put(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn put_empty<B: serde::Serialize>(&self, path: &str, body: &B) -> ClientResult<()> {
        let resp = self
            .http
            .put(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await?;
        Self::handle_empty(resp).await
    }

    async fn delete_json<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
        let resp = self
            .http
            .delete(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    async fn delete_with_body<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<()> {
        let resp = self
            .http
            .delete(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await?;
        Self::handle_empty(resp).await
    }

    #[allow(dead_code)]
    async fn delete_with_query<Q: serde::Serialize>(
        &self,
        path: &str,
        query: &Q,
    ) -> ClientResult<()> {
        let resp = self
            .http
            .delete(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .query(query)
            .send()
            .await?;
        Self::handle_empty(resp).await
    }

    async fn post_empty_with_body<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<()> {
        let resp = self
            .http
            .post(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await?;
        Self::handle_empty(resp).await
    }

    #[allow(dead_code)]
    async fn patch_empty<B: serde::Serialize>(&self, path: &str, body: &B) -> ClientResult<()> {
        let resp = self
            .http
            .patch(format!("{}{path}", self.base_url))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await?;
        Self::handle_empty(resp).await
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
    ) -> ClientResult<T> {
        let status = resp.status();
        if status.is_success() {
            let body = resp.json::<T>().await?;
            Ok(body)
        } else {
            let body = resp.text().await.unwrap_or_default();
            let api_error: Option<serde_json::Value> = serde_json::from_str(&body).ok();
            let message = api_error
                .as_ref()
                .and_then(|v| v.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or(&body)
                .to_string();
            Err(ClientError::Api {
                status: status.as_u16(),
                message,
                details: api_error,
            })
        }
    }

    async fn handle_empty(resp: reqwest::Response) -> ClientResult<()> {
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            let api_error: Option<serde_json::Value> = serde_json::from_str(&body).ok();
            let message = api_error
                .as_ref()
                .and_then(|v| v.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or(&body)
                .to_string();
            Err(ClientError::Api {
                status: status.as_u16(),
                message,
                details: api_error,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Constructor / base_url ----

    #[test]
    fn new_stores_base_url() {
        let client = FxClient::new("http://localhost:3000/api");
        assert_eq!(client.base_url(), "http://localhost:3000/api");
    }

    #[test]
    fn new_strips_trailing_slash() {
        let client = FxClient::new("http://localhost:3000/api/");
        assert_eq!(client.base_url(), "http://localhost:3000/api");
    }

    #[test]
    fn new_strips_multiple_trailing_slashes() {
        let client = FxClient::new("http://example.com///");
        // trim_end_matches('/') removes all trailing slashes
        assert_eq!(client.base_url(), "http://example.com");
    }

    #[test]
    fn new_accepts_string_owned() {
        let url = String::from("https://dzming.li/api");
        let client = FxClient::new(url);
        assert_eq!(client.base_url(), "https://dzming.li/api");
    }

    #[test]
    fn new_no_token_by_default() {
        let client = FxClient::new("http://localhost:3000");
        assert!(client.token.is_none());
        assert!(client.admin_secret.is_none());
    }

    // ---- with_token ----

    #[test]
    fn with_token_sets_token() {
        let client = FxClient::new("http://localhost:3000").with_token("abc123");
        assert_eq!(client.token.as_deref(), Some("abc123"));
    }

    #[test]
    fn with_token_chaining() {
        let client = FxClient::new("http://localhost:3000")
            .with_token("first")
            .with_token("second");
        assert_eq!(client.token.as_deref(), Some("second"));
    }

    #[test]
    fn with_token_accepts_string() {
        let token = String::from("my-token");
        let client = FxClient::new("http://localhost:3000").with_token(token);
        assert_eq!(client.token.as_deref(), Some("my-token"));
    }

    // ---- with_admin_secret ----

    #[test]
    fn with_admin_secret_sets_secret() {
        let client = FxClient::new("http://localhost:3000").with_admin_secret("s3cret");
        assert_eq!(client.admin_secret.as_deref(), Some("s3cret"));
    }

    #[test]
    fn with_admin_secret_chaining() {
        let client = FxClient::new("http://localhost:3000")
            .with_admin_secret("old")
            .with_admin_secret("new");
        assert_eq!(client.admin_secret.as_deref(), Some("new"));
    }

    // ---- set_token / clear_token ----

    #[test]
    fn set_token_updates_token() {
        let mut client = FxClient::new("http://localhost:3000");
        assert!(client.token.is_none());
        client.set_token("tok1");
        assert_eq!(client.token.as_deref(), Some("tok1"));
        client.set_token("tok2");
        assert_eq!(client.token.as_deref(), Some("tok2"));
    }

    #[test]
    fn clear_token_removes_token() {
        let mut client = FxClient::new("http://localhost:3000").with_token("abc");
        assert!(client.token.is_some());
        client.clear_token();
        assert!(client.token.is_none());
    }

    #[test]
    fn clear_token_is_idempotent() {
        let mut client = FxClient::new("http://localhost:3000");
        client.clear_token();
        assert!(client.token.is_none());
        client.clear_token();
        assert!(client.token.is_none());
    }

    // ---- combined builder ----

    #[test]
    fn builder_chain_token_and_admin_secret() {
        let client = FxClient::new("http://localhost:3000")
            .with_token("bearer-tok")
            .with_admin_secret("admin-key");
        assert_eq!(client.token.as_deref(), Some("bearer-tok"));
        assert_eq!(client.admin_secret.as_deref(), Some("admin-key"));
        assert_eq!(client.base_url(), "http://localhost:3000");
    }

    // ---- auth_headers ----

    #[test]
    fn auth_headers_empty_when_no_credentials() {
        let client = FxClient::new("http://localhost:3000");
        let headers = client.auth_headers();
        assert!(headers.is_empty());
    }

    #[test]
    fn auth_headers_includes_bearer_token() {
        let client = FxClient::new("http://localhost:3000").with_token("mytoken");
        let headers = client.auth_headers();
        let auth = headers.get("authorization").expect("should have Authorization");
        assert_eq!(auth.to_str().unwrap(), "Bearer mytoken");
    }

    #[test]
    fn auth_headers_includes_admin_secret() {
        let client = FxClient::new("http://localhost:3000").with_admin_secret("secret123");
        let headers = client.auth_headers();
        let admin = headers.get("x-admin-secret").expect("should have x-admin-secret");
        assert_eq!(admin.to_str().unwrap(), "secret123");
    }

    #[test]
    fn auth_headers_includes_both() {
        let client = FxClient::new("http://localhost:3000")
            .with_token("tok")
            .with_admin_secret("sec");
        let headers = client.auth_headers();
        assert!(headers.get("authorization").is_some());
        assert!(headers.get("x-admin-secret").is_some());
    }

    #[test]
    fn auth_headers_after_clear_token() {
        let mut client = FxClient::new("http://localhost:3000")
            .with_token("tok")
            .with_admin_secret("sec");
        client.clear_token();
        let headers = client.auth_headers();
        assert!(headers.get("authorization").is_none());
        assert!(headers.get("x-admin-secret").is_some());
    }

    // ---- URL construction (verified via format! pattern used internally) ----

    #[test]
    fn url_construction_no_double_slash() {
        let client = FxClient::new("http://localhost:3000/api/");
        // base_url should not have trailing slash
        let url = format!("{}{}", client.base_url(), "/articles");
        assert_eq!(url, "http://localhost:3000/api/articles");
        assert!(!url.contains("//articles"));
    }

    #[test]
    fn url_construction_with_path_params() {
        let client = FxClient::new("http://localhost:3000/api");
        let series_id = "ser-123";
        let url = format!("{}/series/{series_id}/articles", client.base_url());
        assert_eq!(url, "http://localhost:3000/api/series/ser-123/articles");
    }

    #[test]
    fn url_construction_special_characters_in_path() {
        let client = FxClient::new("http://localhost:3000/api");
        let book_id = "book-with-spaces and stuff";
        let url = format!("{}/books/{book_id}", client.base_url());
        // The format! itself doesn't encode, but we verify the concat is correct
        assert!(url.contains("book-with-spaces and stuff"));
    }

    // ---- Clone ----

    #[test]
    fn client_is_cloneable() {
        let client = FxClient::new("http://localhost:3000")
            .with_token("tok")
            .with_admin_secret("sec");
        let cloned = client.clone();
        assert_eq!(cloned.base_url(), client.base_url());
        assert_eq!(cloned.token, client.token);
        assert_eq!(cloned.admin_secret, client.admin_secret);
    }

    #[test]
    fn clone_is_independent() {
        let mut client = FxClient::new("http://localhost:3000").with_token("tok");
        let mut cloned = client.clone();
        cloned.set_token("different");
        assert_eq!(client.token.as_deref(), Some("tok"));
        assert_eq!(cloned.token.as_deref(), Some("different"));

        client.clear_token();
        assert!(client.token.is_none());
        assert_eq!(cloned.token.as_deref(), Some("different"));
    }

    // ---- Debug ----

    #[test]
    fn client_implements_debug() {
        let client = FxClient::new("http://localhost:3000").with_token("secret-tok");
        let debug = format!("{client:?}");
        assert!(debug.contains("FxClient"));
        assert!(debug.contains("http://localhost:3000"));
    }

    // ---- Serialization round-trips for request/response types ----

    mod serde_tests {
        use crate::api::articles::*;
        use crate::api::auth::*;
        use crate::api::books::*;
        use crate::api::drafts::*;
        use crate::api::series::*;
        use crate::api::tags::*;
        use std::collections::HashMap;

        // ---- Articles ----

        #[test]
        fn create_article_input_serialization() {
            let input = CreateArticleInput {
                title: "Test Title".into(),
                summary: Some("desc".into()),
                content: "# Hello".into(),
                content_format: "typst".into(),
                lang: Some("zh".into()),
                license: None,
                translation_of: None,
                restricted: Some(true),
                category: None,
                book_id: None,
                edition_id: None,
                tags: vec!["math".into(), "cs".into()],
                prereqs: vec![ArticlePrereqInput {
                    tag_id: "tag-1".into(),
                    prereq_type: "required".into(),
                }],
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "Test Title");
            assert_eq!(json["content_format"], "typst");
            assert_eq!(json["tags"], serde_json::json!(["math", "cs"]));
            assert_eq!(json["restricted"], true);
            // skip_serializing_if = None fields should be absent
            assert!(json.get("license").is_none());
            assert!(json.get("translation_of").is_none());
            assert!(json.get("category").is_none());
            assert!(json.get("book_id").is_none());
            assert!(json.get("edition_id").is_none());
        }

        #[test]
        fn create_article_input_minimal_serialization() {
            let input = CreateArticleInput {
                title: "T".into(),
                summary: None,
                content: "body".into(),
                content_format: "markdown".into(),
                lang: None,
                license: None,
                translation_of: None,
                restricted: None,
                category: None,
                book_id: None,
                edition_id: None,
                tags: vec![],
                prereqs: vec![],
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "T");
            assert!(json.get("summary").is_none());
            assert!(json.get("lang").is_none());
            assert_eq!(json["tags"], serde_json::json!([]));
        }

        #[test]
        fn update_article_input_omits_none_fields() {
            let input = UpdateArticleInput {
                uri: "at://did:plc:abc/com.example.article/tid123".into(),
                title: None,
                summary: Some("updated".into()),
                content: None,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["uri"], "at://did:plc:abc/com.example.article/tid123");
            assert!(json.get("title").is_none());
            assert_eq!(json["summary"], "updated");
            assert!(json.get("content").is_none());
        }

        #[test]
        fn convert_input_roundtrip() {
            let input = ConvertInput {
                content: "\\section{Hi}".into(),
                from: "latex".into(),
                to: "typst".into(),
            };
            let json_str = serde_json::to_string(&input).unwrap();
            // ConvertInput only derives Serialize, but we can verify JSON structure
            let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            assert_eq!(val["from"], "latex");
            assert_eq!(val["to"], "typst");
        }

        #[test]
        fn convert_output_deserialization() {
            let json = r#"{"content":"= Hi"}"#;
            let output: ConvertOutput = serde_json::from_str(json).unwrap();
            assert_eq!(output.content, "= Hi");
        }

        #[test]
        fn article_deserialization() {
            let json = serde_json::json!({
                "at_uri": "at://did:plc:abc/com.example.article/tid",
                "did": "did:plc:abc",
                "author_handle": "alice",
                "kind": "article",
                "title": "Test",
                "summary": "",
                "content_hash": null,
                "content_format": "typst",
                "lang": "en",
                "translation_group": null,
                "license": "CC-BY-4.0",
                "prereq_threshold": 0.8,
                "category": "cs",
                "question_uri": null,
                "book_id": null,
                "edition_id": null,
                "answer_count": 0,
                "restricted": false,
                "vote_score": 5,
                "bookmark_count": 2,
                "created_at": "2025-01-01T00:00:00Z",
                "updated_at": "2025-06-15T12:30:00Z"
            });
            let article: Article = serde_json::from_value(json).unwrap();
            assert_eq!(article.title, "Test");
            assert_eq!(article.lang, "en");
            assert_eq!(article.vote_score, 5);
            assert!(!article.restricted);
            assert!(article.content_hash.is_none());
            assert_eq!(article.prereq_threshold, 0.8);
        }

        #[test]
        fn article_content_roundtrip() {
            let json = serde_json::json!({
                "source": "= Hello\nworld",
                "html": "<h1>Hello</h1><p>world</p>"
            });
            let content: ArticleContent = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(content.source, "= Hello\nworld");
            let re_serialized = serde_json::to_value(&content).unwrap();
            assert_eq!(re_serialized, json);
        }

        #[test]
        fn article_vote_summary_roundtrip() {
            let json = serde_json::json!({ "score": 10, "upvotes": 15, "downvotes": 5 });
            let votes: ArticleVoteSummary = serde_json::from_value(json).unwrap();
            assert_eq!(votes.score, 10);
            assert_eq!(votes.upvotes, 15);
            assert_eq!(votes.downvotes, 5);
        }

        #[test]
        fn article_prereq_row_with_names_map() {
            let json = serde_json::json!({
                "tag_id": "linear-algebra",
                "prereq_type": "required",
                "tag_name": "Linear Algebra",
                "tag_names": { "en": "Linear Algebra", "zh": "线性代数" }
            });
            let row: ArticlePrereqRow = serde_json::from_value(json).unwrap();
            assert_eq!(row.tag_names.get("zh").unwrap(), "线性代数");
        }

        #[test]
        fn image_upload_response_deserialization() {
            let json = r#"{"filename":"abc123.png"}"#;
            let resp: ImageUploadResponse = serde_json::from_str(json).unwrap();
            assert_eq!(resp.filename, "abc123.png");
        }

        // ---- Auth ----

        #[test]
        fn login_input_serialization() {
            let input = LoginInput {
                identifier: "alice".into(),
                password: "pass123".into(),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["identifier"], "alice");
            assert_eq!(json["password"], "pass123");
        }

        #[test]
        fn login_output_deserialization() {
            let json = serde_json::json!({
                "token": "jwt-token-here",
                "did": "did:plc:abc",
                "handle": "alice",
                "display_name": "Alice",
                "avatar": null
            });
            let output: LoginOutput = serde_json::from_value(json).unwrap();
            assert_eq!(output.token, "jwt-token-here");
            assert_eq!(output.handle, "alice");
            assert_eq!(output.display_name.as_deref(), Some("Alice"));
            assert!(output.avatar.is_none());
        }

        #[test]
        fn login_output_with_avatar() {
            let json = serde_json::json!({
                "token": "t",
                "did": "did:plc:abc",
                "handle": "bob",
                "display_name": null,
                "avatar": "https://example.com/avatar.jpg"
            });
            let output: LoginOutput = serde_json::from_value(json).unwrap();
            assert!(output.display_name.is_none());
            assert_eq!(output.avatar.as_deref(), Some("https://example.com/avatar.jpg"));
        }

        #[test]
        fn auth_me_deserialization() {
            let json = serde_json::json!({
                "did": "did:plc:xyz",
                "handle": "charlie",
                "display_name": "Charlie",
                "avatar": null,
                "is_banned": false,
                "ban_reason": null
            });
            let me: AuthMe = serde_json::from_value(json).unwrap();
            assert_eq!(me.handle, "charlie");
            assert!(!me.is_banned);
            assert!(me.ban_reason.is_none());
        }

        #[test]
        fn auth_me_banned_user() {
            let json = serde_json::json!({
                "did": "did:plc:bad",
                "handle": "spammer",
                "display_name": null,
                "avatar": null,
                "is_banned": true,
                "ban_reason": "spam"
            });
            let me: AuthMe = serde_json::from_value(json).unwrap();
            assert!(me.is_banned);
            assert_eq!(me.ban_reason.as_deref(), Some("spam"));
        }

        // ---- Books ----

        #[test]
        fn create_book_input_serialization() {
            let input = CreateBookInput {
                title: "SICP".into(),
                authors: vec!["Abelson".into(), "Sussman".into()],
                description: Some("Classic CS book".into()),
                cover_url: None,
                tags: Some(vec!["cs".into()]),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "SICP");
            assert_eq!(json["authors"], serde_json::json!(["Abelson", "Sussman"]));
            assert!(json.get("cover_url").is_none());
            assert_eq!(json["tags"], serde_json::json!(["cs"]));
        }

        #[test]
        fn create_book_input_minimal() {
            let input = CreateBookInput {
                title: "T".into(),
                authors: vec![],
                description: None,
                cover_url: None,
                tags: None,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert!(json.get("description").is_none());
            assert!(json.get("cover_url").is_none());
            assert!(json.get("tags").is_none());
        }

        #[test]
        fn book_deserialization() {
            let json = serde_json::json!({
                "id": "book-1",
                "title": "SICP",
                "authors": ["Abelson"],
                "description": "CS classic",
                "cover_url": null,
                "created_by": "did:plc:abc",
                "created_at": "2025-03-01T00:00:00Z"
            });
            let book: Book = serde_json::from_value(json).unwrap();
            assert_eq!(book.id, "book-1");
            assert_eq!(book.authors, vec!["Abelson"]);
            assert!(book.cover_url.is_none());
        }

        #[test]
        fn book_chapter_roundtrip() {
            let json = serde_json::json!({
                "id": "ch-1",
                "book_id": "book-1",
                "parent_id": null,
                "title": "Chapter 1",
                "order_index": 0,
                "article_uri": "at://did:plc:x/app.article/tid1"
            });
            let chapter: BookChapter = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(chapter.title, "Chapter 1");
            assert!(chapter.parent_id.is_none());
            let reserialized = serde_json::to_value(&chapter).unwrap();
            assert_eq!(reserialized, json);
        }

        #[test]
        fn book_rating_stats_deserialization() {
            let json = serde_json::json!({ "avg_rating": 8.5, "rating_count": 42 });
            let stats: BookRatingStats = serde_json::from_value(json).unwrap();
            assert!((stats.avg_rating - 8.5).abs() < f64::EPSILON);
            assert_eq!(stats.rating_count, 42);
        }

        #[test]
        fn rate_book_input_serialization() {
            let input = RateBookInput {
                book_id: "book-1".into(),
                rating: 9,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["rating"], 9);
        }

        #[test]
        fn set_reading_status_input_serialization() {
            let input = SetReadingStatusInput {
                book_id: "book-1".into(),
                status: "reading".into(),
                progress: 50,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["status"], "reading");
            assert_eq!(json["progress"], 50);
        }

        #[test]
        fn chapter_progress_input_serialization() {
            let input = ChapterProgressInput {
                book_id: "b".into(),
                chapter_id: "ch".into(),
                completed: true,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["completed"], true);
        }

        #[test]
        fn add_edition_input_serialization() {
            let input = AddEditionInput {
                book_id: "b1".into(),
                title: "2nd Ed".into(),
                lang: "en".into(),
                isbn: Some("978-0-123456-78-9".into()),
                publisher: None,
                year: Some("2020".into()),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["isbn"], "978-0-123456-78-9");
            assert!(json.get("publisher").is_none());
            assert_eq!(json["year"], "2020");
        }

        #[test]
        fn create_chapter_input_serialization() {
            let input = CreateChapterInput {
                book_id: "b1".into(),
                chapter: CreateChapterData {
                    title: "Intro".into(),
                    parent_id: None,
                    order_index: 0,
                    article_uri: Some("at://did:plc:x/a/t".into()),
                },
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["chapter"]["title"], "Intro");
            assert!(json["chapter"].get("parent_id").is_none());
        }

        #[test]
        fn update_book_input_partial() {
            let input = UpdateBookInput {
                id: "b1".into(),
                title: Some("New Title".into()),
                description: None,
                cover_url: None,
                edit_summary: Some("Fixed typo".into()),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "New Title");
            assert!(json.get("description").is_none());
            assert_eq!(json["edit_summary"], "Fixed typo");
        }

        #[test]
        fn book_edit_log_deserialization() {
            let json = serde_json::json!({
                "id": "log-1",
                "book_id": "b1",
                "editor_did": "did:plc:x",
                "editor_handle": "alice",
                "old_data": {"title": "Old"},
                "new_data": {"title": "New"},
                "summary": "Title update",
                "created_at": "2025-06-01T00:00:00Z"
            });
            let log: BookEditLog = serde_json::from_value(json).unwrap();
            assert_eq!(log.summary, "Title update");
            assert_eq!(log.old_data["title"], "Old");
        }

        #[test]
        fn reading_status_deserialization() {
            let json = serde_json::json!({
                "book_id": "b1",
                "user_did": "did:plc:x",
                "status": "finished",
                "progress": 100,
                "updated_at": "2025-06-01T00:00:00Z"
            });
            let rs: ReadingStatus = serde_json::from_value(json).unwrap();
            assert_eq!(rs.status, "finished");
            assert_eq!(rs.progress, 100);
        }

        #[test]
        fn chapter_progress_deserialization() {
            let json = serde_json::json!({
                "book_id": "b1",
                "chapter_id": "ch1",
                "user_did": "did:plc:x",
                "completed": true,
                "completed_at": "2025-06-01T12:00:00Z"
            });
            let cp: ChapterProgress = serde_json::from_value(json).unwrap();
            assert!(cp.completed);
            assert!(cp.completed_at.is_some());
        }

        // ---- Drafts ----

        #[test]
        fn save_draft_input_serialization() {
            let input = SaveDraftInput {
                title: "My Draft".into(),
                summary: None,
                content: "WIP content".into(),
                content_format: "typst".into(),
                lang: Some("zh".into()),
                license: None,
                tags: vec!["math".into()],
                prereqs: vec![],
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "My Draft");
            assert!(json.get("summary").is_none());
            assert_eq!(json["lang"], "zh");
            assert!(json.get("license").is_none());
        }

        #[test]
        fn update_draft_input_all_none() {
            let input = UpdateDraftInput {
                id: "draft-1".into(),
                title: None,
                summary: None,
                content: None,
                content_format: None,
                lang: None,
                license: None,
                tags: None,
                prereqs: None,
            };
            let json = serde_json::to_value(&input).unwrap();
            // Only id should be present
            assert_eq!(json["id"], "draft-1");
            assert!(json.get("title").is_none());
            assert!(json.get("content").is_none());
            assert!(json.get("tags").is_none());
        }

        #[test]
        fn update_draft_input_partial() {
            let input = UpdateDraftInput {
                id: "d1".into(),
                title: Some("Updated".into()),
                summary: None,
                content: Some("new body".into()),
                content_format: None,
                lang: None,
                license: None,
                tags: Some(vec!["physics".into()]),
                prereqs: None,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "Updated");
            assert_eq!(json["content"], "new body");
            assert_eq!(json["tags"], serde_json::json!(["physics"]));
        }

        #[test]
        fn draft_deserialization() {
            let json = serde_json::json!({
                "id": "draft-1",
                "did": "did:plc:abc",
                "title": "WIP",
                "summary": "",
                "content": "hello",
                "content_format": "markdown",
                "lang": "en",
                "license": "CC-BY-4.0",
                "tags": "math,cs",
                "prereqs": "[]",
                "at_uri": null,
                "created_at": "2025-01-01T00:00:00Z",
                "updated_at": "2025-01-02T00:00:00Z"
            });
            let draft: Draft = serde_json::from_value(json).unwrap();
            assert_eq!(draft.id, "draft-1");
            assert_eq!(draft.content_format, "markdown");
            assert!(draft.at_uri.is_none());
        }

        // ---- Series ----

        #[test]
        fn create_series_input_serialization() {
            let input = CreateSeriesInput {
                title: "Linear Algebra Series".into(),
                summary: Some("Learn LA".into()),
                long_description: None,
                topics: Some(vec!["math".into()]),
                parent_id: None,
                lang: Some("zh".into()),
                translation_of: None,
                category: Some("math".into()),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "Linear Algebra Series");
            assert_eq!(json["summary"], "Learn LA");
            assert!(json.get("long_description").is_none());
            assert_eq!(json["topics"], serde_json::json!(["math"]));
            assert!(json.get("parent_id").is_none());
            assert_eq!(json["category"], "math");
        }

        #[test]
        fn create_series_input_minimal() {
            let input = CreateSeriesInput {
                title: "S".into(),
                summary: None,
                long_description: None,
                topics: None,
                parent_id: None,
                lang: None,
                translation_of: None,
                category: None,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["title"], "S");
            // All optional fields should be absent
            for key in [
                "summary",
                "long_description",
                "topics",
                "parent_id",
                "lang",
                "translation_of",
                "category",
            ] {
                assert!(json.get(key).is_none(), "{key} should be absent");
            }
        }

        #[test]
        fn series_row_deserialization() {
            let json = serde_json::json!({
                "id": "s1",
                "title": "My Series",
                "summary": "Desc",
                "long_description": null,
                "parent_id": null,
                "order_index": 0,
                "created_by": "did:plc:abc",
                "created_at": "2025-01-01T00:00:00Z",
                "lang": "en",
                "translation_group": null,
                "category": "cs"
            });
            let row: SeriesRow = serde_json::from_value(json).unwrap();
            assert_eq!(row.id, "s1");
            assert_eq!(row.category, "cs");
        }

        #[test]
        fn series_list_row_deserialization() {
            let json = serde_json::json!({
                "id": "s1",
                "title": "Series",
                "summary": null,
                "long_description": null,
                "parent_id": null,
                "order_index": 0,
                "created_by": "did:plc:x",
                "author_handle": "alice",
                "created_at": "2025-01-01T00:00:00Z",
                "lang": "zh",
                "translation_group": null,
                "category": "math",
                "article_count": 5,
                "child_count": 2
            });
            let row: SeriesListRow = serde_json::from_value(json).unwrap();
            assert_eq!(row.article_count, 5);
            assert_eq!(row.child_count, 2);
            assert_eq!(row.author_handle.as_deref(), Some("alice"));
        }

        #[test]
        fn series_article_member_row_roundtrip() {
            let row = SeriesArticleMemberRow {
                series_id: "s1".into(),
                article_uri: "at://did:plc:x/app.article/tid".into(),
            };
            let json = serde_json::to_value(&row).unwrap();
            let deserialized: SeriesArticleMemberRow = serde_json::from_value(json).unwrap();
            assert_eq!(deserialized.series_id, "s1");
            assert_eq!(deserialized.article_uri, row.article_uri);
        }

        #[test]
        fn series_context_item_deserialization() {
            let json = serde_json::json!({
                "series_id": "s1",
                "series_title": "My Series",
                "total": 10,
                "prev": [{"uri": "at://a/b/c", "title": "Prev"}],
                "next": []
            });
            let item: SeriesContextItem = serde_json::from_value(json).unwrap();
            assert_eq!(item.total, 10);
            assert_eq!(item.prev.len(), 1);
            assert!(item.next.is_empty());
        }

        // ---- Tags ----

        #[test]
        fn create_tag_input_serialization() {
            let input = CreateTagInput {
                id: "linear-algebra".into(),
                name: "Linear Algebra".into(),
                names: Some(HashMap::from([
                    ("en".into(), "Linear Algebra".into()),
                    ("zh".into(), "线性代数".into()),
                ])),
                description: Some("Study of linear equations".into()),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["id"], "linear-algebra");
            assert_eq!(json["names"]["zh"], "线性代数");
        }

        #[test]
        fn create_tag_input_minimal() {
            let input = CreateTagInput {
                id: "t1".into(),
                name: "Tag One".into(),
                names: None,
                description: None,
            };
            let json = serde_json::to_value(&input).unwrap();
            assert!(json.get("names").is_none());
            assert!(json.get("description").is_none());
        }

        #[test]
        fn update_tag_names_input_serialization() {
            let input = UpdateTagNamesInput {
                id: "calculus".into(),
                names: HashMap::from([("en".into(), "Calculus".into()), ("zh".into(), "微积分".into())]),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["id"], "calculus");
            assert_eq!(json["names"]["zh"], "微积分");
        }

        #[test]
        fn set_teach_input_serialization() {
            let input = SetTeachInput {
                content_uri: "at://did:plc:x/a/t".into(),
                tag_id: "math".into(),
            };
            let json = serde_json::to_value(&input).unwrap();
            assert_eq!(json["content_uri"], "at://did:plc:x/a/t");
            assert_eq!(json["tag_id"], "math");
        }

        #[test]
        fn tag_deserialization() {
            let json = serde_json::json!({
                "id": "calculus",
                "name": "Calculus",
                "names": { "en": "Calculus", "zh": "微积分" },
                "description": "Differential and integral calculus",
                "created_by": "did:plc:abc",
                "created_at": "2025-01-01T00:00:00Z"
            });
            let tag: Tag = serde_json::from_value(json).unwrap();
            assert_eq!(tag.id, "calculus");
            assert_eq!(tag.names.len(), 2);
            assert_eq!(tag.description.as_deref(), Some("Differential and integral calculus"));
        }

        #[test]
        fn tag_with_empty_names_map() {
            let json = serde_json::json!({
                "id": "t",
                "name": "T",
                "names": {},
                "description": null,
                "created_by": "did:plc:x",
                "created_at": "2025-01-01T00:00:00Z"
            });
            let tag: Tag = serde_json::from_value(json).unwrap();
            assert!(tag.names.is_empty());
            assert!(tag.description.is_none());
        }

        // ---- Deserialization error cases ----

        #[test]
        fn article_missing_required_field() {
            let json = serde_json::json!({
                "at_uri": "x",
                "did": "x"
                // missing many required fields
            });
            let result = serde_json::from_value::<Article>(json);
            assert!(result.is_err());
        }

        #[test]
        fn login_output_missing_token() {
            let json = serde_json::json!({
                "did": "x",
                "handle": "x",
                "display_name": null,
                "avatar": null
                // missing "token"
            });
            let result = serde_json::from_value::<LoginOutput>(json);
            assert!(result.is_err());
        }

        #[test]
        fn book_wrong_type_for_authors() {
            let json = serde_json::json!({
                "id": "b",
                "title": "T",
                "authors": "not an array",
                "description": "",
                "cover_url": null,
                "created_by": "did:plc:x",
                "created_at": "2025-01-01T00:00:00Z"
            });
            let result = serde_json::from_value::<Book>(json);
            assert!(result.is_err());
        }
    }
}
