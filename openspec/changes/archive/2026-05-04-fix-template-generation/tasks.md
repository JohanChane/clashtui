## 1. Core Implementation

- [x] 1.1 Add `template_proxy_providers_path()` accessor in `src/config.rs`
- [x] 1.2 Refactor `gen_template()` to take `template_name: &str` and read `template_proxy_providers` URLs
- [x] 1.3 Expand proxy-providers: for each `tpl_param` entry, create N copies with `url` + `path` set
- [x] 1.4 Expand proxy-groups: for each `tpl_param.providers` entry, create one group per generated provider
- [x] 1.5 Resolve `<AngleBracket>` placeholders in group `use` and `proxies` lists
- [x] 1.6 Remove `clashtui: null` marker injection
- [x] 1.7 Update `apply_template()` to pass `template_name` to `gen_template()`
- [x] 2.1 Rewrite `gen_template` tests to use internal `gen_template_with_urls(tpl, template_name, &[String])` for testability
- [x] 2.2 Update `simple_tpl_output.yaml`: expand `pvd` → `pvd0` with URL + path
- [x] 2.3 Update `multi_provider_tpl_output.yaml`: expand multiple providers with URLs
- [x] 2.4 Update `no_tpl_param_tpl_output.yaml`: ensure no `clashtui` key
- [x] 2.5 Update `empty_uses_tpl_output.yaml`: match new behavior
- [x] 2.6 Add test: single provider with multiple URLs expansion
- [x] 2.7 Add test: missing `template_proxy_providers` file (empty URLs) → no tpl_param entries
- [x] 2.8 Add test: output has no `clashtui` key

## 3. Verification

- [x] 3.1 Run `cargo test` — all tests pass
- [x] 3.2 Run `cargo check` — no warnings
