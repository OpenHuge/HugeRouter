# HugeCode Browser Proxy Distribution Runbook

## Scope

HugeRouter owns the real upstream proxy configuration for HugeCode embedded browser traffic.
HugeCode should only carry the Router base URL and a client proxy token. Do not package the proxy
host, proxy password, or fixed upstream IP into the HugeCode installer.

## Runtime Flow

```text
HugeCode startup/open browser
  -> GET https://<router-domain>/v1/client/browser-proxy
  -> Authorization: Bearer <CLIENT_BROWSER_PROXY_TOKEN>
  -> HugeRouter returns SOCKS proxy config
  -> HugeCode local HTTP bridge connects to upstream SOCKS
```

The client still creates a local bridge inside HugeCode. Chromium talks to the local bridge, and the
bridge talks to the upstream SOCKS endpoint returned by HugeRouter.

## HugeRouter Environment

Configure these on the Router server. Do not commit real values.

```dotenv
CLIENT_BROWSER_PROXY_TOKEN=
CLIENT_BROWSER_PROXY_SCHEME=socks5
CLIENT_BROWSER_PROXY_HOST=
CLIENT_BROWSER_PROXY_PORT=
CLIENT_BROWSER_PROXY_USERNAME=
CLIENT_BROWSER_PROXY_PASSWORD=
CLIENT_BROWSER_PROXY_CONNECT_HOST=
CLIENT_BROWSER_PROXY_BYPASS_RULES=<local>;localhost;127.0.0.1;::1
```

Required:

- `CLIENT_BROWSER_PROXY_TOKEN`
- `CLIENT_BROWSER_PROXY_HOST`
- `CLIENT_BROWSER_PROXY_PORT`
- `CLIENT_BROWSER_PROXY_USERNAME`
- `CLIENT_BROWSER_PROXY_PASSWORD`

Optional:

- `CLIENT_BROWSER_PROXY_SCHEME`: defaults to `socks5`.
- `CLIENT_BROWSER_PROXY_CONNECT_HOST`: fixed trusted IP used only when DNS/TUN/fake-ip pollution must be bypassed.
- `CLIENT_BROWSER_PROXY_BYPASS_RULES`: defaults to local bypass rules.

## `connectHost` Policy

`CLIENT_BROWSER_PROXY_CONNECT_HOST` is not required. Use it only when the relay domain is being
resolved incorrectly on user machines, for example to a Clash fake-ip range such as `198.18.x.x`.

Behavior:

```text
TCP connect target = CLIENT_BROWSER_PROXY_CONNECT_HOST if set
TCP connect target = CLIENT_BROWSER_PROXY_HOST otherwise
```

Keep `CLIENT_BROWSER_PROXY_HOST` as the normal relay host label even when `CONNECT_HOST` is set.

## HugeCode Packaged Config

HugeCode should package only:

```json
{
  "baseUrl": "https://<router-domain>",
  "clientBrowserProxyToken": "<same value as CLIENT_BROWSER_PROXY_TOKEN>"
}
```

Do not package:

- `CLIENT_BROWSER_PROXY_HOST`
- `CLIENT_BROWSER_PROXY_USERNAME`
- `CLIENT_BROWSER_PROXY_PASSWORD`
- `CLIENT_BROWSER_PROXY_CONNECT_HOST`

## Smoke Test

After deploying the Router code and environment variables:

```bash
curl -sS \
  -H "Authorization: Bearer $CLIENT_BROWSER_PROXY_TOKEN" \
  https://<router-domain>/v1/client/browser-proxy
```

Expected JSON shape:

```json
{
  "version": 1,
  "scheme": "socks5",
  "host": "relay.example.com",
  "port": 34072,
  "username": "hugeproxy",
  "password": "redacted",
  "connect_host": "203.0.113.10",
  "bypass_rules": "<local>;localhost;127.0.0.1;::1"
}
```

If `connect_host` is not configured, it is omitted.

## User-Side Proxy/TUN

HugeCode uses its own managed proxy. If the user has Clash, VPN, TUN, system proxy, or fake-ip mode
enabled, it can still interfere with HugeCode connecting to the upstream proxy. The preferred product
behavior is to ask the user to close those tools before loading ChatGPT in the embedded browser.
