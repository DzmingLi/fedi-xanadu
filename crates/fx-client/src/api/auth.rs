//! Auth API: login, logout, me.

use serde::{Deserialize, Serialize};

use crate::{ClientResult, FxClient};

#[derive(Debug, Clone, Serialize)]
pub struct LoginInput {
    pub identifier: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginOutput {
    pub token: String,
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthMe {
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub is_banned: bool,
    pub ban_reason: Option<String>,
}

impl FxClient {
    /// Log in with handle/password. Returns session info including token.
    pub async fn login(&self, identifier: &str, password: &str) -> ClientResult<LoginOutput> {
        self.post(
            "/auth/login",
            &LoginInput {
                identifier: identifier.to_string(),
                password: password.to_string(),
            },
        )
        .await
    }

    /// Log in and automatically set the token on this client.
    pub async fn login_and_set_token(
        &mut self,
        identifier: &str,
        password: &str,
    ) -> ClientResult<LoginOutput> {
        let output = self.login(identifier, password).await?;
        self.set_token(&output.token);
        Ok(output)
    }

    /// Log out the current session.
    pub async fn logout(&self) -> ClientResult<()> {
        self.post_empty("/auth/logout").await
    }

    /// Get the current authenticated user's info.
    pub async fn me(&self) -> ClientResult<AuthMe> {
        self.get("/auth/me").await
    }
}
