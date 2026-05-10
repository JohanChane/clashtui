## Context

demotui currently updates profiles by downloading the profile YAML from a URL and saving it to disk. However, clash configuration YAML files often contain embedded network resource references in `proxy-providers` and `rule-providers` sections — each with their own `url` and `path` fields. These sub-resources are not currently detected or downloaded.

clashtui has this capability via `extract_net_providers()` / `extract_net_provider_helper()`. We need to port a simplified version (without GitHub/Gitee token handling for sub-resources) into demotui.

Current flow (per `src/functions/file/profile.rs:80-145`):
```
update_profile(profile)
  → load_local_profile()
  → match dtype:
      Url(url)     → update_with() → download::profile(url)
      Generated()  → apply_template() + optional PP removal
      Github{...}  → update_with() → download::github(url, token)
      GitLab{...}  → update_with() → download::gitlab(url, token)
  → return single-line result string
```

The `update_with()` closure (`profile.rs:45-78`) downloads, validates YAML (checks for `proxies` or `proxy-providers` key), and writes to disk — but it never inspects the content further.

## Goals / Non-Goals

**Goals:**
- Define a `NetworkResource` type describing a discovered resource (name, url, path, section)
- Define an `ExtractNetResources` trait to extract network resources from parsed YAML
- Walk `proxy-providers` and `rule-providers` sections for entries with `url` and `path` fields
- Integrate extraction into the `update_profile()` flow after the YAML is downloaded and written
- Download each extracted resource to its target path (relative to clash config dir)
- Display per-resource update results with domain-only URLs (not raw URLs, for security)

**Non-Goals:**
- GitHub/Gitee/GitLab token-aware URL handling for sub-resources (profile-level token auth already exists via `ProfileType::Github`/`GitLab`)
- Template-based profile resource extraction (Generated profiles use template PP URLs)
- Changing the popup UI framework (reuse existing `Confirm` popup)
- Rule-provider specific update semantics (just download, like proxy-providers)

## Decisions

### 1. Data types live in `src/functions/file/net_resource.rs`

A new file dedicated to network resource extraction, keeping the concern separate from the update flow orchestration.

```rust
pub enum ResourceSection {
    ProxyProvider,
    RuleProvider,
}

pub struct NetResource {
    pub name: String,
    pub url: String,
    pub path: String,      // relative path within clash config dir
    pub section: ResourceSection,
}
```

The `NetResource` struct is owned data (no lifetimes), suitable for passing across async boundaries.

### 2. Trait-based extraction: `ExtractNetResources`

```rust
pub trait ExtractNetResources {
    fn extract(&self, sections: &[ResourceSection]) -> Vec<NetResource>;
}
```

Implemented for `serde_yml::Mapping` — this keeps the trait generic and testable without needing file I/O.

Implementation walks the mapping looking for `"proxy-providers"` and `"rule-providers"` keys. For each entry in these sections that has both `"url"` and `"path"` fields, a `NetResource` is constructed. The URL is taken as-is (plain string, no token injection).

### 3. Integration into `update_profile()`

The extraction is added to the `update_with()` closure, **after** the YAML is downloaded and written to disk, because:
- The profile file itself needs to be saved first (so subsequent reloads see the updated profile)
- Provider downloads go to paths relative to `clash_cfg_dir`, which may not exist yet — we create parent dirs on demand
- If provider downloads fail, the profile is still valid (stale providers are better than no profile)

Modified flow:
```
update_with(url, name, path, with_proxy, apply):
  1. Download profile YAML → validate → write to path         (existing)
  2. Parse YAML content → extract NetResources                 (NEW)
  3. For each NetResource:                                     (NEW)
     a. download::profile(resource.url, with_proxy)
     b. Write to clash_cfg_dir / resource.path
     c. Record result (domain-only URL in output)
  4. Return multi-line result string                           (CHANGED)
```

### 4. URL privacy in output

Update results show `domain` only instead of raw URLs, matching clashtui's behavior. Example output:

```
Updated: my-profile(example.com)
  Updated: proxy0(cdn.example.net)
  Not Updated: proxy1(timeout.example.org)
```

The existing `extract_domain()` helper in `profile.rs:160-169` is reused for domain extraction.

### 5. No token handling for sub-resources

We deliberately skip GitHub/Gitee token-aware URLs. Most proxy-provider URLs are CDN/raw URLs that don't need tokens. If a proxy provider URL happens to need auth, it will fail with an HTTP error, and the result will include "Not Updated" with the domain. This is acceptable for the initial implementation.

## Risks / Trade-offs

- **Download failures cascade**: If one provider fails to download, the others still proceed. The profile YAML is already saved. → Mitigation: Each provider download is independent; failures are recorded as "Not Updated" lines.
- **Large profiles with many providers**: A profile with 50+ proxy providers could take a long time to update. → Mitigation: Use reasonable timeout (reuse `DEFAULT_TIMEOUT = 5s`), downloads are sequential but each is fast.
- **Missing clash config dir**: Provider paths are relative to `clash_cfg_dir`; if the clash core isn't running, the dir might not exist. → Mitigation: `create_dir_all()` before writing each provider file.
