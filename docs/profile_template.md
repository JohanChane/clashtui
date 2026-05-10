# Profile Template

Templates let you define a parameterized clash configuration that expands into a full profile YAML. Template proxy-providers carry their own URLs — no external URL list is needed. Generation simply strips `tpl_param` markers and expands `<>` placeholders.

## Overview

A template is a standard Clash YAML file with two extra features:

1. **`tpl_param`** markers on `proxy-providers` and `proxy-groups` entries — these entries are templates that expand at generation time
2. **`<>`** angle-bracket placeholders in `use` and `proxies` lists — these reference template entries and expand to all generated names

URLs come from the **template proxy-provider entries themselves**. Each `tpl_param` proxy-provider must include a `url` field. No external URL list or database lookup is needed.

When you generate a template, demotui reads the template YAML, removes all `tpl_param` markers, expands `<>` placeholders, and writes the result to `profile_yamls/<profile_name>.yaml`.

**Output YAML preserves the section ordering of the input template.** Entries appear in the same relative order — template entries keep their original position.

## Template YAML Format

```yaml
# Required sections:
proxy-providers:    # Mapping — at least one entry (with url for tpl_param entries)
proxy-groups:       # Sequence — at least one entry

# Passthrough sections (copied as-is):
rules:
rule-providers:
# ... any other clash config keys
```

Templates are **pure clash YAML**. No `clashtui.uses` section is needed — each proxy-provider provides its own URL directly.

## Proxy-Provider Template Entries

A proxy-provider entry with a `tpl_param` key is a **template provider**. At generation time, `tpl_param` is removed and the entry passes through with all other fields intact.

### Input template
```yaml
proxy-providers:
  pvd:                      # Template entry — has tpl_param
    tpl_param:              # Marker (empty value)
    type: http
    interval: 3600
    url: https://example.com/sub1.yaml
    health-check:
      enable: true
      url: https://www.gstatic.com/generate_204
      interval: 300
  static:                   # Passthrough entry — no tpl_param
    type: http
    interval: 3600
    url: https://static.example.com/proxy.yaml
    path: ./proxy-providers/static.yaml
```

### After generation
```yaml
proxy-providers:
  pvd:                      # tpl_param removed, url kept from template
    type: http
    interval: 3600
    url: https://example.com/sub1.yaml
    health-check:
      enable: true
      url: https://www.gstatic.com/generate_204
      interval: 300
  static:                   # Passthrough preserved in place
    type: http
    interval: 3600
    url: https://static.example.com/proxy.yaml
    path: ./proxy-providers/static.yaml
```

Key behaviors:
- `tpl_param` is removed from generated entries
- All other fields (url, path, type, interval, health-check, etc.) are kept as-is from the template
- Entries without `tpl_param` pass through unchanged
- The entry keeps its original name and position (**ordering guarantee**)

## Proxy-Group Template Entries

A proxy-group entry with `tpl_param.providers` is a **template group**. It generates one group per matching proxy-provider.

### Input template
```yaml
proxy-groups:
  - name: Select           # Passthrough — no tpl_param
    type: select
    proxies:
      - DIRECT
      - <Auto>             # Placeholder — expands to all Auto-* groups
  - name: Auto             # Template group — has tpl_param
    type: url-test
    tpl_param:
      providers:           # Generate one group per provider matched here
        - pvd
    url: https://www.gstatic.com/generate_204
    interval: 300
  - name: Direct           # Passthrough — no tpl_param
    type: select
    proxies:
      - DIRECT
```

### After generation
```yaml
proxy-groups:
  - name: Select
    type: select
    proxies:
      - DIRECT
      - Auto-pvd           # <Auto> expanded
  - name: Auto-pvd         # Generated from Auto template
    type: url-test
    url: https://www.gstatic.com/generate_204
    interval: 300
    use:
      - pvd
  - name: Direct           # Passthrough preserved
    type: select
    proxies:
      - DIRECT
```

Key behaviors:
- Generated group names follow the pattern `{group_name}-{provider_name}` (e.g., `Auto-pvd`)
- The `use` field is set to the specific provider name
- The `tpl_param` key is removed from generated entries
- Non-template groups pass through unchanged
- Group ordering is preserved; template groups expand in place

### Edge case: no matching providers

If `tpl_param.providers` references a provider name that has no expanded instances, that group template generates **no entries** and is silently skipped. The group is removed from output.

## `<>` Placeholder Expansion

Angle-bracket placeholders in `use` and `proxies` lists expand to all matching generated names.

| Placeholder | Expands in | Expands to |
|-------------|-----------|------------|
| `<pvd>` | `use` | All generated proxy-provider names with that key (e.g., `pvd`) |
| `<Auto>` | `proxies` | All generated proxy-group names with that prefix (e.g., `Auto-pvd`) |

```yaml
# Before:
proxy-groups:
  - name: Entry
    type: select
    use:
      - <pvd>         # Expands to pvd
    proxies:
      - DIRECT
      - <Auto>        # Expands to Auto-pvd

# After (pvd→[pvd], Auto→[Auto-pvd]):
proxy-groups:
  - name: Entry
    type: select
    use:
      - pvd
    proxies:
      - DIRECT
      - Auto-pvd
```

Non-bracket values pass through unchanged.

If a placeholder references a non-existent target, generation fails with an error.

## Profile Storage

demotui uses two directories under the config root:

| Directory | Purpose |
|-----------|---------|
| `templates/` | Template YAML files with `tpl_param` markers |
| `profile_yamls/` | All profile YAML (generated, imported, and downloaded) |

**Profile types** in the database (`clashtui.db`):

| Type | Description |
|------|-------------|
| `File` | Local file imported to `profile_yamls/`, or generated from template |
| `Url` | Downloaded from a subscription URL |

Template-generated profiles are registered as `ProfileType::File` — they are indistinguishable from imported files. Legacy `!Template` and `!Generated` entries auto-migrate to `!File` on load.

## File Path Import

You can import a local clash YAML configuration by filesystem path:

1. Switch to the **Profile** tab
2. Press `I` (shift-i) to import from file
3. Enter a profile name
4. Enter the source file path

The file is copied to `profile_yamls/<name>.yaml` and registered as `ProfileType::File`.

## Full Workflow

```
┌──────────────────────────────────────────────────────────────────┐
│  1. CREATE TEMPLATE                                              │
│     Write a clash YAML file with tpl_param markers and URLs.      │
│     Place in config directory: templates/my-template.yaml        │
├──────────────────────────────────────────────────────────────────┤
│  2. GENERATE PROFILE                                             │
│     In Template tab, select template → press Enter.               │
│     Creates profile_yamls/<name>.generated.yaml                  │
│     Registered as ProfileType::File in the database.             │
├──────────────────────────────────────────────────────────────────┤
│  3. SELECT PROFILE                                               │
│     In Profile pane, select the profile → press Enter.           │
│     demotui merges it with basic_clash_config.yaml and deploys   │
│     to the clash config path.                                    │
├──────────────────────────────────────────────────────────────────┤
│  4. UPDATE PROFILE                                               │
│     Press 'u' to re-read the file from profile_yamls/.           │
│     Press 'a' 'u' to update all profiles.                        │
└──────────────────────────────────────────────────────────────────┘
```

## Complete Example

### Template: `templates/my-config.yaml`
```yaml
proxy-providers:
  pvd:
    tpl_param:
    type: http
    interval: 3600
    url: https://example.com/sub1.yaml
    health-check:
      enable: true
      url: https://www.gstatic.com/generate_204
      interval: 300
proxy-groups:
  - name: Entry
    type: select
    proxies:
      - DIRECT
      - <Auto>
      - REJECT
  - name: Auto
    type: url-test
    tpl_param:
      providers:
        - pvd
    url: https://www.gstatic.com/generate_204
    interval: 300
  - name: Direct
    type: select
    proxies:
      - DIRECT
rules:
  - DOMAIN-SUFFIX,google.com,Entry
  - MATCH,DIRECT
```

### Generated output: `profile_yamls/my-config.yaml`
```yaml
proxy-providers:
  pvd:
    type: http
    interval: 3600
    url: https://example.com/sub1.yaml
    health-check:
      enable: true
      url: https://www.gstatic.com/generate_204
      interval: 300
proxy-groups:
  - name: Entry
    type: select
    proxies:
      - DIRECT
      - Auto-pvd
      - REJECT
  - name: Auto-pvd
    type: url-test
    url: https://www.gstatic.com/generate_204
    interval: 300
    use:
      - pvd
  - name: Direct
    type: select
    proxies:
      - DIRECT
rules:
  - DOMAIN-SUFFIX,google.com,Entry
  - MATCH,DIRECT
clashtui: null
```
