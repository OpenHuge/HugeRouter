# Example Payloads

These fixtures are generated from the stable `v1` contract slice and are meant
to be consumed by backend, frontend, and documentation tests.

Current coverage:

- control-plane list and read responses
- route simulation request and response payloads
- gateway chat request, response, and normalized error payloads
- event envelope examples for `usage_event.recorded` and `config_snapshot.activated`

Refresh them with:

```sh
pnpm generate
```
