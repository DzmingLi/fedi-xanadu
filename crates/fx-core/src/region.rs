/// Instance deployment mode. Set once at startup via config, not per-request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstanceMode {
    /// China mainland instance — requires phone verification for writes.
    Cn,
    /// International instance — AT Protocol login, no phone requirement.
    #[default]
    Intl,
}

impl InstanceMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "cn" => InstanceMode::Cn,
            _ => InstanceMode::Intl,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            InstanceMode::Cn => "cn",
            InstanceMode::Intl => "intl",
        }
    }

    /// Whether this instance requires phone verification for write operations.
    pub fn requires_phone(&self) -> bool {
        matches!(self, InstanceMode::Cn)
    }
}

/// Default visibility for new articles based on author's phone verification.
/// Verified → `public` (visible everywhere). Unverified → `cn_hidden` (intl only).
pub fn default_visibility(phone_verified: bool) -> &'static str {
    if phone_verified { "public" } else { "cn_hidden" }
}

/// SQL WHERE fragment for article visibility on this instance.
///
/// CN: only `public`. Intl: `public` + `cn_hidden`.
pub fn visibility_filter(mode: InstanceMode) -> &'static str {
    match mode {
        InstanceMode::Cn => "a.visibility = 'public'",
        InstanceMode::Intl => "a.visibility IN ('public', 'cn_hidden')",
    }
}
