use sdkwork_lr_proxy::AuthInjection;

pub const SUPPORTED_UPSTREAM_AUTH_SCHEMES: &[&str] =
    &["bearer", "x-api-key", "x-goog-api-key", "query-key"];

pub fn canonical_upstream_auth_scheme(scheme: &str) -> Option<&'static str> {
    match scheme
        .trim()
        .replace('-', "_")
        .to_ascii_lowercase()
        .as_str()
    {
        "" => Some(""),
        "bearer" | "authorization_bearer" => Some("bearer"),
        "x_api_key" | "anthropic_x_api_key" => Some("x-api-key"),
        "x_goog_api_key" | "google_api_key" => Some("x-goog-api-key"),
        "query_key" | "key_query" | "google_query_key" => Some("query-key"),
        _ => None,
    }
}

pub fn auth_from_scheme(scheme: Option<&str>, upstream_api_key: &str) -> Option<AuthInjection> {
    let scheme = canonical_upstream_auth_scheme(scheme?)?;
    if scheme.is_empty() {
        return None;
    }

    match scheme {
        "bearer" => Some(AuthInjection::Bearer(upstream_api_key.to_owned())),
        "x-api-key" => Some(AuthInjection::Header(
            "x-api-key".to_owned(),
            upstream_api_key.to_owned(),
        )),
        "x-goog-api-key" => Some(AuthInjection::Header(
            "x-goog-api-key".to_owned(),
            upstream_api_key.to_owned(),
        )),
        "query-key" => Some(AuthInjection::Query(
            "key".to_owned(),
            upstream_api_key.to_owned(),
        )),
        _ => None,
    }
}

pub fn is_supported_auth_scheme(scheme: Option<&str>) -> bool {
    let Some(scheme) = scheme else {
        return true;
    };
    let scheme = scheme.trim();
    if scheme.is_empty() {
        return true;
    }
    canonical_upstream_auth_scheme(scheme).is_some()
}
