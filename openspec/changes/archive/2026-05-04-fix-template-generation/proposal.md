## Why

`gen_template()` does not expand proxy-providers with `tpl_param` using URLs from `template_proxy_providers`. It merely strips the `tpl_param` marker and passes through the single template entry unchanged. Template-generated profiles are `ProfileType::File` — the proxy-provider URLs must be baked into the generated YAML at generation time, not deferred to clash runtime. Additionally, an unnecessary `clashtui: null` marker is injected into output.

## What Changes

- `gen_template()` reads `template_proxy_providers` file (one URL per line, skip `#` comments and blank lines)
- For each proxy-provider entry with `tpl_param`, creates N copies (one per URL), each with `url` set and `path` set to `proxy-providers/tpl/<template_name>/<key><idx>.yaml`
- Proxy-groups with `tpl_param.providers` expand one group per generated provider named `<group>-<providerN>`
- `<AngleBracket>` placeholders in group `use` and `proxies` lists resolve to expanded names
- Remove `clashtui: null` marker injection from output
- Update testdata YAML files to reflect expansion behavior

## Capabilities

### New Capabilities
- `template-generation`: Transform template YAML (with `tpl_param` markers + `template_proxy_providers` URLs) into a complete profile YAML with expanded proxy-providers and proxy-groups

### Modified Capabilities
- `network-resource-extraction`: Generated proxy-provider entries now always have `url` + `path` set, so `extract_net_resources()` will find them

## Impact

- `src/functions/file/template/version1.rs` — `gen_template()` signature changes (needs `template_name: &str` parameter), core expansion logic rewritten to match clashtui
- `src/functions/file/template.rs` — `apply_template()` passes `template_name` to `gen_template()`
- `src/functions/file/template/testdata/` — output YAML files updated for new behavior (expansion, no `clashtui: null`)
- `src/config.rs` — `template_proxy_providers` path constant or accessor needed
