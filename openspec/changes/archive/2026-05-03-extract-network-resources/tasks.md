## 1. Define network resource types

- [x] 1.1 Create `src/functions/file/net_resource.rs` with `ResourceSection` enum (ProxyProvider, RuleProvider)
- [x] 1.2 Define `NetResource` struct with fields: `name: String`, `url: String`, `path: String`, `section: ResourceSection`
- [x] 1.3 Define `ExtractNetResources` trait with method `fn extract(&self, sections: &[ResourceSection]) -> Vec<NetResource>`
- [x] 1.4 Register module: add `mod net_resource;` to `src/functions/file.rs`
- [x] 1.5 Run `cargo check` to confirm module structure compiles

## 2. Implement YAML extraction

- [x] 2.1 Implement `ExtractNetResources` for `serde_yml::Mapping`
- [x] 2.2 Walk `"proxy-providers"` key: iterate entries, extract `url` and `path` from each mapping entry
- [x] 2.3 Walk `"rule-providers"` key: same extraction logic
- [x] 2.4 Respect section filter: only process requested section types
- [x] 2.5 Skip entries missing `url` or `path` fields silently
- [x] 2.6 Handle edge cases: non-mapping values in provider sections, scalar URLs, empty sections

## 3. Integrate extraction into update flow

- [x] 3.1 In `src/functions/file/profile.rs`, modify `update_with()` to accept a `clash_cfg_dir` parameter for resolving provider paths
- [x] 3.2 After writing the downloaded profile YAML to disk, parse it and call `extract()` on the content
- [x] 3.3 For each extracted `NetResource`, download its URL via `download::profile()` and write to `<clash_cfg_dir>/<path>`
- [x] 3.4 Create parent directories for each provider target path before writing
- [x] 3.5 Collect per-resource results: "Updated: name(domain)" on success, "Not Updated: name(domain)" on failure

## 4. Update output display

- [x] 4.1 Change `update_with()` return type from `String` to `Vec<String>` (multi-line results)
- [x] 4.2 Profile update result is the first line, sub-resource results are indented lines ("  ...")
- [x] 4.3 Use `extract_domain()` for all URL display (never show raw URLs)
- [x] 4.4 Update `update_profile()` to handle `Vec<String>` return and join into single string for TUI display
- [x] 4.5 The existing `Confirm` popup display in `actions::update()` and `actions::update_all()` continues to work (it already handles multi-line strings)

## 5. Test data and test cases

- [x] 5.1 Create `src/functions/file/testdata/net_resource_test.yaml` containing:
  - 2 `proxy-providers` entries (e.g. `pp-dcdn` with `https://cdn.example.com/dcdn.yaml` → `./proxy-providers/dcdn.yaml`, `pp-aws` with `https://s3.amazonaws.com/bucket/proxies.yaml` → `./proxy-providers/aws.yaml`)
  - 2 `rule-providers` entries (e.g. `rp-reject` with `https://rules.example.org/reject.yaml` → `./rule-providers/reject.yaml`, `rp-ads` with `https://filters.example.net/ads.yaml` → `./rule-providers/ads.yaml`)
  - 1 proxy-provider entry with **no `url`** field (should be skipped)
  - 1 rule-provider entry with **no `path`** field (should be skipped)
  - 1 non-provider section (`proxies`) that should be ignored
- [x] 5.2 Write `#[test]` cases in `net_resource.rs` that:
  - load the test YAML file via `serde_yml::from_reader` → verify extraction finds exactly 2 proxy-providers + 2 rule-providers
  - test extraction with filter `[ProxyProvider]` only → returns 2 resources, all `ResourceSection::ProxyProvider`
  - test extraction with filter `[RuleProvider]` only → returns 2 resources, all `ResourceSection::RuleProvider`
  - test extraction with filter `[ProxyProvider, RuleProvider]` → returns 4 resources total
  - test empty filter `[]` → returns 0 resources
  - test each extracted resource's `name`, `url`, `path`, `section` fields for correctness
  - test YAML with no provider sections at all → returns empty vec
  - test YAML where `proxy-providers` is a scalar (not a mapping) → returns empty vec

## 6. Verify

- [x] 6.1 Run `cargo check` to ensure type correctness
- [x] 6.2 Run `cargo test` to verify unit tests pass
- [x] 6.3 Run `cargo build` for a full debug build
