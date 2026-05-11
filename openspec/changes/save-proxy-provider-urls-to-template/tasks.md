## 1. Database Schema Changes

- [x] 1.1 Remove `proxy_provider_groups` field from `ProfileType::Template` variant in `src/config/database.rs`
- [x] 1.2 Update `Serialize` impl for `ProfileType` — `Template` variant serializes only `template: String`
- [x] 1.3 Update `Deserialize` impl with legacy migration — `Wire::Template` accepts optional `proxy_provider_groups`, writes groups to template file, logs warning
- [x] 1.4 Update `Generated` → `Template` migration (line 106-114) — new variant signature without groups
- [x] 1.5 Update `ProfileManager::insert()` match arms referencing `Template { .. }` destructuring
- [x] 1.6 Update all database tests in `src/config/database.rs` to new `Template { template }` format

## 2. Template File Proxy Provider Groups Read/Write

- [x] 2.1 Add `read_template_ppg(template_name: &str) -> anyhow::Result<ProxyProviderGroups>` — reads `clashtui.proxy_provider_groups` from a template file in `templates/`
- [x] 2.2 Add `read_profile_ppg(profile_name: &str) -> anyhow::Result<ProxyProviderGroups>` — reads `clashtui.proxy_provider_groups` from a generated profile in `profiles/`
- [x] 2.3 Add `write_template_ppg(template_name: &str, groups: &ProxyProviderGroups) -> anyhow::Result<()>` — writes/merges `clashtui.proxy_provider_groups` into template file preserving other keys
- [x] 2.4 Remove `read_template_proxy_providers()` (the old standalone-file reader)
- [x] 2.5 ~~Add unit tests for `read_template_ppg`, `read_profile_ppg`, and `write_template_ppg`~~ (covered by integration tests for template generation)

## 3. Template Generation Updates

- [x] 3.1 Update `apply_template()` to read groups via `read_template_ppg(template_name)` instead of receiving them as a parameter
- [x] 3.2 Update `apply_template_singbox()` to read groups via `read_template_ppg(template_name)` instead of receiving them as a parameter
- [x] 3.3 Update `version1::gen_template()` to inject `clashtui.proxy_provider_groups` into generated output (instead of `clashtui: null`)
- [x] 3.4 Update `singbox::gen_template_singbox()` to inject `clashtui.proxy_provider_groups` into generated output
- [x] 3.5 Update `pm.insert()` calls in `apply_template()` and `apply_template_singbox()` to `ProfileType::Template { template }` without groups

## 4. Profile Update and Selection Flow

- [x] 4.1 Update `update_template_profile()` — read groups from generated profile via `read_profile_ppg()` instead of `profile.dtype`; remove `apply_template()` call at end
- [x] 4.2 For Mihomo: download all proxy-provider URLs to cache dir, record statuses, but do NOT regenerate
- [x] 4.3 For sing-box: download proxy-provider subscription content, record statuses, but do NOT call `apply_template_singbox()`
- [x] 4.4 After successful update, if updated profile is the currently active profile, trigger a re-select so the core reloads with updated proxy-provider files
- [x] 4.5 Add `check_template_ppg_availability(profile: &Profile) -> anyhow::Result<()>` — verify all proxy-provider files exist before selection
- [x] 4.6 Integrate `check_template_ppg_availability()` into the profile select flow before using a template-generated profile

## 5. TUI Template Tab Updates

- [x] 5.1 Update `generate()` in `src/tui/tab/files/template.rs` — remove `read_template_proxy_providers()` call, let `apply_template()` fetch groups internally
- [x] 5.2 Update `_edit_providers()` to open the template file itself (which now contains `clashtui.proxy_provider_groups`) instead of the standalone YAML
- [x] 5.3 ~~Update all other call sites of `read_template_proxy_providers()`~~ (function removed, all call sites updated)

## 6. Cleanup

- [x] 6.1 Remove `TEMPLATE_PROXY_PROVIDERS_FILE` constant from `src/config/util.rs`
- [x] 6.2 Remove `template_proxy_providers_path()` helper from `src/config.rs`
- [x] 6.3 Remove `singbox_template_proxy_providers_path()` helper from `src/config.rs`
- [x] 6.4 Run `cargo test` — verify all tests pass (151 passed, 0 failed)
- [x] 6.5 Run `cargo check` and `cargo build` — verified no errors
