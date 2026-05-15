use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::{bearer_token_from_headers, ApiError, ControlPlaneState, RequestContext};

const DEFAULT_CLIENT_BROWSER_PROXY_BYPASS_RULES: &str = "<local>;localhost;127.0.0.1;::1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientBrowserProxyProfileResponse {
    pub id: String,
    pub label: String,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_host: Option<String>,
    pub bypass_rules: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientBrowserProxyResponse {
    pub version: u32,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_host: Option<String>,
    pub bypass_rules: String,
    pub default_profile_id: String,
    pub profiles: Vec<ClientBrowserProxyProfileResponse>,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientBrowserProxyConfig {
    pub response: ClientBrowserProxyResponse,
    pub token: String,
}

pub fn client_browser_proxy_config_from_lookup(
    mut lookup: impl FnMut(&str) -> Option<String>,
) -> Option<ClientBrowserProxyConfig> {
    let token = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_TOKEN")?;
    let profiles_json = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_PROFILES_JSON");
    let default_profile_id = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_DEFAULT_PROFILE_ID");

    if let Some(profiles_json) = profiles_json {
        let profiles = serde_json::from_str::<Vec<ClientBrowserProxyProfileResponse>>(&profiles_json)
            .ok()?
            .into_iter()
            .filter(|profile| {
                !profile.id.trim().is_empty()
                    && !profile.label.trim().is_empty()
                    && !profile.scheme.trim().is_empty()
                    && !profile.host.trim().is_empty()
                    && !profile.username.trim().is_empty()
                    && !profile.password.trim().is_empty()
                    && profile.port > 0
            })
            .collect::<Vec<_>>();
        let default_profile_id = default_profile_id?;
        let default_profile = profiles
            .iter()
            .find(|profile| profile.id == default_profile_id)?;
        return Some(ClientBrowserProxyConfig {
            response: ClientBrowserProxyResponse {
                version: 1,
                scheme: default_profile.scheme.clone(),
                host: default_profile.host.clone(),
                port: default_profile.port,
                username: default_profile.username.clone(),
                password: default_profile.password.clone(),
                connect_host: default_profile.connect_host.clone(),
                bypass_rules: default_profile.bypass_rules.clone(),
                default_profile_id,
                profiles,
            },
            token,
        });
    }

    let host = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_HOST")?;
    let port = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_PORT")?
        .parse::<u16>()
        .ok()
        .filter(|value| *value > 0)?;
    let username = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_USERNAME")?;
    let password = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_PASSWORD")?;
    let scheme = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_SCHEME")
        .unwrap_or_else(|| "socks5".to_string());
    let connect_host = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_CONNECT_HOST");
    let bypass_rules = read_lookup_string(&mut lookup, "CLIENT_BROWSER_PROXY_BYPASS_RULES")
        .unwrap_or_else(|| DEFAULT_CLIENT_BROWSER_PROXY_BYPASS_RULES.to_string());

    Some(ClientBrowserProxyConfig {
        response: ClientBrowserProxyResponse {
            default_profile_id: "default".to_string(),
            bypass_rules: bypass_rules.clone(),
            connect_host: connect_host.clone(),
            host: host.clone(),
            password: password.clone(),
            port,
            profiles: vec![ClientBrowserProxyProfileResponse {
                id: "default".to_string(),
                label: "默认线路".to_string(),
                scheme: scheme.clone(),
                host: host.clone(),
                port,
                username: username.clone(),
                password: password.clone(),
                connect_host: connect_host.clone(),
                bypass_rules: bypass_rules.clone(),
            }],
            scheme,
            username,
            version: 1,
        },
        token,
    })
}

pub async fn get_client_browser_proxy(
    State(state): State<ControlPlaneState>,
    headers: HeaderMap,
) -> Result<Json<ClientBrowserProxyResponse>, ApiError> {
    let context = crate::next_request_context();
    let config = state.client_browser_proxy_config.as_ref().ok_or_else(|| {
        ApiError::internal(
            "client_browser_proxy_unconfigured",
            "client browser proxy is not configured".to_string(),
            &context,
        )
    })?;
    authorize_client_browser_proxy(&config, &headers, &context)?;
    Ok(Json(config.response.clone()))
}

fn authorize_client_browser_proxy(
    config: &ClientBrowserProxyConfig,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<(), ApiError> {
    let token = bearer_token_from_headers(headers, "client_browser_proxy_token_missing", context)?;
    if token == config.token {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "client_browser_proxy_forbidden",
            "client browser proxy token is invalid".to_string(),
            context,
        ))
    }
}

fn read_lookup_string(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    key: &'static str,
) -> Option<String> {
    lookup(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::client_browser_proxy_config_from_lookup;
    use std::collections::HashMap;

    fn config_from(entries: &[(&str, &str)]) -> Option<super::ClientBrowserProxyConfig> {
        let values = entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<HashMap<_, _>>();
        client_browser_proxy_config_from_lookup(|name| values.get(name).cloned())
    }

    #[test]
    fn builds_client_browser_proxy_config_from_environment_values() {
        let config = config_from(&[
            ("CLIENT_BROWSER_PROXY_TOKEN", "client-token"),
            ("CLIENT_BROWSER_PROXY_HOST", "relay.example.com"),
            ("CLIENT_BROWSER_PROXY_PORT", "34072"),
            ("CLIENT_BROWSER_PROXY_USERNAME", "hugeproxy"),
            ("CLIENT_BROWSER_PROXY_PASSWORD", "secret"),
            ("CLIENT_BROWSER_PROXY_CONNECT_HOST", "203.0.113.10"),
        ])
        .expect("proxy config");

        assert_eq!(config.token, "client-token");
        assert_eq!(config.response.scheme, "socks5");
        assert_eq!(config.response.host, "relay.example.com");
        assert_eq!(config.response.port, 34072);
        assert_eq!(config.response.username, "hugeproxy");
        assert_eq!(config.response.password, "secret");
        assert_eq!(config.response.connect_host.as_deref(), Some("203.0.113.10"));
        assert_eq!(config.response.default_profile_id, "default");
        assert_eq!(config.response.profiles.len(), 1);
        assert_eq!(config.response.profiles[0].id, "default");
        assert_eq!(
            config.response.bypass_rules,
            "<local>;localhost;127.0.0.1;::1"
        );
    }

    #[test]
    fn builds_client_browser_proxy_profiles_from_environment_values() {
        let config = config_from(&[
            ("CLIENT_BROWSER_PROXY_TOKEN", "client-token"),
            ("CLIENT_BROWSER_PROXY_DEFAULT_PROFILE_ID", "asia-fast-1"),
            (
                "CLIENT_BROWSER_PROXY_PROFILES_JSON",
                r#"[
                  {
                    "id":"asia-fast-1",
                    "label":"亚洲高速1",
                    "scheme":"socks5",
                    "host":"relay-a.example.com",
                    "port":2081,
                    "username":"user-a",
                    "password":"secret-a",
                    "connect_host":"43.134.121.38",
                    "bypass_rules":"<local>;localhost;127.0.0.1;::1"
                  },
                  {
                    "id":"asia-fast-2",
                    "label":"亚洲高速2",
                    "scheme":"socks5",
                    "host":"relay-b.example.com",
                    "port":2081,
                    "username":"user-b",
                    "password":"secret-b",
                    "connect_host":"43.134.121.39",
                    "bypass_rules":"<local>;localhost;127.0.0.1;::1"
                  }
                ]"#,
            ),
        ])
        .expect("proxy config");

        assert_eq!(config.token, "client-token");
        assert_eq!(config.response.default_profile_id, "asia-fast-1");
        assert_eq!(config.response.profiles.len(), 2);
        assert_eq!(config.response.profiles[0].id, "asia-fast-1");
        assert_eq!(config.response.profiles[1].id, "asia-fast-2");
        assert_eq!(config.response.host, "relay-a.example.com");
        assert_eq!(config.response.port, 2081);
    }

    #[test]
    fn invalid_default_profile_id_disables_profile_config() {
        assert!(
            config_from(&[
                ("CLIENT_BROWSER_PROXY_TOKEN", "client-token"),
                ("CLIENT_BROWSER_PROXY_DEFAULT_PROFILE_ID", "missing-profile"),
                (
                    "CLIENT_BROWSER_PROXY_PROFILES_JSON",
                    r#"[
                      {
                        "id":"asia-fast-1",
                        "label":"亚洲高速1",
                        "scheme":"socks5",
                        "host":"relay-a.example.com",
                        "port":2081,
                        "username":"user-a",
                        "password":"secret-a",
                        "bypass_rules":"<local>;localhost;127.0.0.1;::1"
                      }
                    ]"#,
                ),
            ])
            .is_none()
        );
    }

    #[test]
    fn missing_required_client_browser_proxy_values_disable_config() {
        assert!(config_from(&[]).is_none());
        assert!(
            config_from(&[
                ("CLIENT_BROWSER_PROXY_TOKEN", "client-token"),
                ("CLIENT_BROWSER_PROXY_HOST", "relay.example.com"),
                ("CLIENT_BROWSER_PROXY_PORT", "34072"),
                ("CLIENT_BROWSER_PROXY_USERNAME", "hugeproxy"),
            ])
            .is_none()
        );
    }
}
