use serde::{Deserialize, Serialize};

/// AT Protocol PDS client for authentication and record operations.
#[derive(Clone)]
pub struct AtClient {
    http: reqwest::Client,
}

// --- Auth types ---

#[derive(Debug, Serialize)]
pub struct CreateSessionInput {
    pub identifier: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub did: String,
    pub handle: String,
    #[serde(rename = "accessJwt")]
    pub access_jwt: String,
    #[serde(rename = "refreshJwt")]
    pub refresh_jwt: Option<String>,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub did: String,
    pub handle: String,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

// --- Record types ---

#[derive(Debug, Serialize)]
pub struct CreateRecordInput {
    pub repo: String,
    pub collection: String,
    pub record: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rkey: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecordOutput {
    pub uri: String,
    pub cid: String,
}

#[derive(Debug, Serialize)]
pub struct PutRecordInput {
    pub repo: String,
    pub collection: String,
    pub rkey: String,
    pub record: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct PutRecordOutput {
    pub uri: String,
    pub cid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRef {
    #[serde(rename = "$type")]
    pub blob_type: String,
    #[serde(rename = "ref")]
    pub ref_link: BlobLink,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobLink {
    #[serde(rename = "$link")]
    pub link: String,
}

#[derive(Debug, Deserialize)]
struct UploadBlobOutput {
    blob: BlobRef,
}

#[derive(Debug, Serialize)]
pub struct DeleteRecordInput {
    pub repo: String,
    pub collection: String,
    pub rkey: String,
}

// --- Resolve handle ---

#[derive(Debug, Deserialize)]
struct ResolveHandleOutput {
    did: String,
}

impl Default for AtClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AtClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .connect_timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    /// Resolve a handle (e.g. "alice.bsky.social") to the PDS service URL.
    /// Returns (did, pds_url).
    pub async fn resolve_handle(&self, handle: &str) -> anyhow::Result<(String, String)> {
        // Try resolving via public API first
        let url = format!(
            "https://bsky.social/xrpc/com.atproto.identity.resolveHandle?handle={}",
            handle
        );
        let resp: ResolveHandleOutput = self.http.get(&url).send().await?.json().await?;
        let did = resp.did;

        // Get PDS URL from DID document
        let pds_url = self.get_pds_url(&did).await?;
        Ok((did, pds_url))
    }

    /// Get the PDS service endpoint from a DID's DID document.
    async fn get_pds_url(&self, did: &str) -> anyhow::Result<String> {
        let url = if did.starts_with("did:plc:") {
            format!("https://plc.directory/{}", did)
        } else if did.starts_with("did:web:") {
            let host = did.strip_prefix("did:web:").unwrap();
            format!("https://{}/.well-known/did.json", host)
        } else {
            anyhow::bail!("unsupported DID method: {}", did);
        };

        let doc: serde_json::Value = self.http.get(&url).send().await?.json().await?;

        // Look for atproto_pds service endpoint
        if let Some(services) = doc.get("service").and_then(|s| s.as_array()) {
            for svc in services {
                let svc_type = svc.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if svc_type == "AtprotoPersonalDataServer"
                    && let Some(endpoint) = svc.get("serviceEndpoint").and_then(|e| e.as_str())
                {
                    return Ok(endpoint.to_string());
                }
            }
        }

        anyhow::bail!("no PDS service endpoint found for {}", did);
    }

    /// Create a session on the user's PDS (login with handle + app password).
    pub async fn create_session(
        &self,
        pds_url: &str,
        identifier: &str,
        password: &str,
    ) -> anyhow::Result<Session> {
        let url = format!(
            "{}/xrpc/com.atproto.server.createSession",
            pds_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .json(&CreateSessionInput {
                identifier: identifier.to_string(),
                password: password.to_string(),
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("createSession failed ({}): {}", status, body);
        }

        Ok(resp.json().await?)
    }

    /// Fetch user profile from PDS.
    pub async fn get_profile(&self, pds_url: &str, did: &str, token: &str) -> anyhow::Result<Profile> {
        let url = format!(
            "{}/xrpc/app.bsky.actor.getProfile?actor={}",
            pds_url.trim_end_matches('/'),
            did
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !resp.status().is_success() {
            // Fallback: return basic profile from session info
            anyhow::bail!("getProfile failed");
        }

        Ok(resp.json().await?)
    }

    /// Fetch a public AT Protocol profile (no auth required).
    /// Uses the Bluesky public API.
    pub async fn get_public_profile(&self, did: &str) -> anyhow::Result<Profile> {
        let url = format!(
            "https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor={}",
            did
        );
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("public getProfile failed: {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    /// Create a record in the user's PDS repository.
    pub async fn create_record(
        &self,
        pds_url: &str,
        token: &str,
        input: &CreateRecordInput,
    ) -> anyhow::Result<CreateRecordOutput> {
        let url = format!(
            "{}/xrpc/com.atproto.repo.createRecord",
            pds_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(input)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("createRecord failed: {}", body);
        }

        Ok(resp.json().await?)
    }

    /// Create or update a record in the user's PDS repository.
    pub async fn put_record(
        &self,
        pds_url: &str,
        token: &str,
        input: &PutRecordInput,
    ) -> anyhow::Result<PutRecordOutput> {
        let url = format!(
            "{}/xrpc/com.atproto.repo.putRecord",
            pds_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(input)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("putRecord failed: {}", body);
        }

        Ok(resp.json().await?)
    }

    /// Upload a blob to the user's PDS.
    pub async fn upload_blob(
        &self,
        pds_url: &str,
        token: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<BlobRef> {
        let url = format!(
            "{}/xrpc/com.atproto.repo.uploadBlob",
            pds_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", content_type)
            .body(data)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("uploadBlob failed: {}", body);
        }

        let output: UploadBlobOutput = resp.json().await?;
        Ok(output.blob)
    }

    /// Delete a record from the user's PDS repository.
    pub async fn delete_record(
        &self,
        pds_url: &str,
        token: &str,
        input: &DeleteRecordInput,
    ) -> anyhow::Result<()> {
        let url = format!(
            "{}/xrpc/com.atproto.repo.deleteRecord",
            pds_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(input)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("deleteRecord failed: {}", body);
        }

        Ok(())
    }
}
