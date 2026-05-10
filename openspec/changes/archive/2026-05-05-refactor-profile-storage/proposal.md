## Why

`clashtui.uses` couples template YAML files to specific URL profiles, making templates less reusable. The URL sourcing belongs at the profile level: different profiles should be able to use the same template with different URLs. Additionally, there's no dedicated directory for clash YAML configurations — generated profiles and imported files need a clear home separate from the source profile database.

## What Changes

- **BREAKING**: Remove `clashtui.uses` from template YAML — templates become pure clash config with `tpl_param` markers only
- **BREAKING**: Extend profile database record to store template name + URL list for template-type profiles (replaces `ProfileType::Generated(String)` as the data carrier for template profiles)
- `gen_template()` now receives URLs directly from the profile record instead of filtering by template-embedded `clashtui.uses`
- Add `profile_yamls/` directory — dedicated storage for clash YAML configurations (generated + imported files)
- Generated profiles write output to `profile_yamls/<name>.yaml` instead of `profiles/<name>.yaml`
- Add file path import support: import a local YAML file by path, copy to `profile_yamls/`, register as a profile
- Rename internal references from `profile_cache` (which currently doesn't exist in code) to `profile_yamls`
- Update `apply_template()` pipeline and `update_profile()` to use the new profile record and output directory
- Update TUI profile tab to support creating/editing template profiles with URL lists

## Capabilities

### New Capabilities

- `template-profile-record`: Profile database record that stores a template name and a list of URLs, enabling profile-level control over which subscriptions feed into template expansion
- `profile-yamls-storage`: Dedicated `profile_yamls/` directory under the config root for storing generated clash YAML and imported file copies
- `file-path-import`: Import a clash YAML configuration from a local filesystem path by copying it into `profile_yamls/` and registering it in the profile database

### Modified Capabilities

- `template-generation`: Remove `clashtui.uses` from template YAML format; URL list is now injected from the profile record instead of being filtered by template-embedded names
- `profile-serialization`: Template-generated profiles write to `profile_yamls/` (not `profiles/`); profile database record format for template-type profiles changes to carry a URL list alongside the template name

## Impact

- Affected code: `src/functions/file/template.rs` (apply_template pipeline), `src/functions/file/template/version1.rs` (gen_template + tests), `src/functions/file/profile.rs` (update_profile, db operations), `src/functions/file/profile/database.rs` (ProfileManager integration), `src/functions/file.rs` (path constants), `src/config/database.rs` (ProfileType schema), `src/config.rs` (directory init), `src/tui/tab/files/profile.rs` (TUI actions)
- Template YAML test data in `src/functions/file/template/testdata/` needs updating: remove `clashtui.uses` blocks from template fixtures
- `docs/profile_template.md` needs updating to reflect the new workflow
- No new dependencies
- Breaking change for existing template YAML files and profile database records
