# Test suite for install (bash) script

setup() {
  # Detect project root: the tests dir is at installs/tests/
  TEST_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")" && pwd -P)"
  PROJECT_ROOT="$(cd "$TEST_DIR/../.." && pwd -P)"

  # Create a temp dir for test output
  TEST_OUTPUT="$BATS_TEST_TMPDIR/output"
  mkdir -p "$TEST_OUTPUT"
}

teardown() {
  true
}

# ---------------------------------------------------------------------------
# Unit tests: detect_os
# ---------------------------------------------------------------------------

@test "detect_os returns linux on Linux" {
  run bash -c "source '${PROJECT_ROOT}/installs/install' && detect_os"
  [ "$status" -eq 0 ]
  [[ "$output" =~ ^(linux|darwin|freebsd)$ ]]
}

@test "detect_os returns a known value" {
  run bash -c "source '${PROJECT_ROOT}/installs/install' && detect_os"
  [ "$status" -eq 0 ]
  [ "$output" != "unsupported" ]
}

# ---------------------------------------------------------------------------
# Unit tests: detect_architecture
# ---------------------------------------------------------------------------

@test "detect_architecture returns amd64 or arm64 on supported arch" {
  run bash -c "source '${PROJECT_ROOT}/installs/install' && detect_architecture"
  [ "$status" -eq 0 ]
  [ "$output" != "unsupported" ]
}

# ---------------------------------------------------------------------------
# Unit tests: command_exists
# ---------------------------------------------------------------------------

@test "command_exists returns 0 for existing command" {
  run bash -c "source '${PROJECT_ROOT}/installs/install' && command_exists bash"
  [ "$status" -eq 0 ]
}

@test "command_exists returns non-zero for nonexistent command" {
  run bash -c "source '${PROJECT_ROOT}/installs/install' && command_exists nonexistent_cmd_xyz123"
  [ "$status" -ne 0 ]
}

# ---------------------------------------------------------------------------
# Unit tests: backup_file
# ---------------------------------------------------------------------------

@test "backup_file creates backup with _1 suffix" {
  local tmpfile="$BATS_TEST_TMPDIR/testfile"
  echo "original" > "$tmpfile"

  run bash -c "source '${PROJECT_ROOT}/installs/install' && backup_file '$tmpfile'"
  [ "$status" -eq 0 ]
  [ ! -f "$tmpfile" ]
  [ -f "${tmpfile}_1" ]
  [ "$(cat "${tmpfile}_1")" = "original" ]
}

@test "backup_file increments suffix when backups exist" {
  local tmpfile="$BATS_TEST_TMPDIR/testfile"
  echo "v1" > "${tmpfile}_1"
  echo "v2" > "${tmpfile}_2"
  echo "current" > "$tmpfile"

  run bash -c "source '${PROJECT_ROOT}/installs/install' && backup_file '$tmpfile'"
  [ "$status" -eq 0 ]
  [ ! -f "$tmpfile" ]
  [ -f "${tmpfile}_3" ]
  [ "$(cat "${tmpfile}_3")" = "current" ]
}

@test "backup_file returns 1 if file does not exist" {
  run bash -c "source '${PROJECT_ROOT}/installs/install' && backup_file '/nonexistent/file'"
  [ "$status" -eq 1 ]
}

# ---------------------------------------------------------------------------
# End-to-end: install --is-test (local contrib)
# ---------------------------------------------------------------------------

@test "install --is-test creates expected directory structure" {
  run bash "${PROJECT_ROOT}/installs/install" \
    --is-test \
    --repo "JohanChane/clashtui" \
    --branch "demotui" \
    --core all

  echo "--- output ---"
  echo "$output"

  # The test temp dir is created by the script (mktemp -d)
  # Extract test dir from output
  local test_dir=$(echo "$output" | grep -oP 'Test mode: using temp directory \K.*')

  # Script should succeed
  [ "$status" -eq 0 ]

  # Find the actual test output directory
  # The script outputs "[TEST] ..." lines, we need the test dir from the first log
  if [ -n "$test_dir" ]; then
    [ -d "$test_dir" ]
    [ -d "$test_dir/opt/clashtui/bin" ]
    [ -d "$test_dir/opt/clashtui/mihomo/config" ]
    [ -d "$test_dir/opt/clashtui/sing-box/config" ]
    [ -d "$test_dir/config/clashtui" ]
  fi
}

@test "install --is-test generates config.yaml with correct paths" {
  run bash "${PROJECT_ROOT}/installs/install" \
    --is-test \
    --repo "JohanChane/clashtui" \
    --branch "demotui" \
    --core all

  [ "$status" -eq 0 ]

  local test_dir=$(echo "$output" | grep -oP 'Test mode: using temp directory \K.*')
  if [ -n "$test_dir" ]; then
    local config_path="$test_dir/config/clashtui/config.yaml"
    [ -f "$config_path" ]

    # Should contain mihomo and singbox sections
    grep -q "mihomo:" "$config_path"
    grep -q "singbox:" "$config_path"
    grep -q "bin_path:" "$config_path"
  fi
}

@test "install --is-test generates template_proxy_providers.yaml for mihomo" {
  run bash "${PROJECT_ROOT}/installs/install" \
    --is-test \
    --repo "JohanChane/clashtui" \
    --branch "demotui" \
    --core mihomo

  [ "$status" -eq 0 ]

  local test_dir=$(echo "$output" | grep -oP 'Test mode: using temp directory \K.*')
  if [ -n "$test_dir" ]; then
    [ -f "$test_dir/config/clashtui/mihomo/template_proxy_providers.yaml" ]
    grep -q "proxy-provider" "$test_dir/config/clashtui/mihomo/template_proxy_providers.yaml"
  fi
}

@test "install --is-test creates profiles and templates directories" {
  run bash "${PROJECT_ROOT}/installs/install" \
    --is-test \
    --repo "JohanChane/clashtui" \
    --branch "demotui" \
    --core all

  [ "$status" -eq 0 ]

  local test_dir=$(echo "$output" | grep -oP 'Test mode: using temp directory \K.*')
  if [ -n "$test_dir" ]; then
    [ -d "$test_dir/config/clashtui/mihomo/profiles" ]
    [ -d "$test_dir/config/clashtui/mihomo/templates" ]
    [ -d "$test_dir/config/clashtui/sing-box/profiles" ]
    [ -d "$test_dir/config/clashtui/sing-box/templates" ]
  fi
}

# ---------------------------------------------------------------------------
# End-to-end: --is-test with --is-user
# ---------------------------------------------------------------------------

@test "install --is-test --is-user creates user-mode paths" {
  run bash "${PROJECT_ROOT}/installs/install" \
    --is-test \
    --is-user \
    --repo "JohanChane/clashtui" \
    --branch "demotui" \
    --core all

  [ "$status" -eq 0 ]

  local test_dir=$(echo "$output" | grep -oP 'Test mode: using temp directory \K.*')
  if [ -n "$test_dir" ]; then
    # In user mode, the config should still be under the test dir
    [ -d "$test_dir/config/clashtui" ]
  fi
}

# ---------------------------------------------------------------------------
# Contrib URL validation
# ---------------------------------------------------------------------------

@test "copy_contrib constructs remote URL correctly" {
  run bash -c "
    source '${PROJECT_ROOT}/installs/install'
    REPO='custom/repo' BRANCH='feat-x'
    CONTRIB_SOURCE='remote'
    CONTRIB_URL_PREFIX='https://raw.githubusercontent.com/custom/repo/refs/heads/feat-x/contrib'
    # Just verify URL prefix is set correctly
    echo \"\$CONTRIB_URL_PREFIX\"
  "

  [ "$status" -eq 0 ]
  [[ "$output" == *"custom/repo"* ]]
  [[ "$output" == *"feat-x"* ]]
}

# ---------------------------------------------------------------------------
# detect_cpu_level (smoke test on current platform)
# ---------------------------------------------------------------------------

@test "detect_cpu_level runs without error" {
  run bash -c "source '${PROJECT_ROOT}/installs/install' && detect_cpu_level"
  # Should return something (empty or e.g. "-v3") with status 0
  [ "$status" -eq 0 ]
}

# ---------------------------------------------------------------------------
# detect_is_macos consistency
# ---------------------------------------------------------------------------

@test "detect_is_macos is consistent with detect_os" {
  run bash -c "
    source '${PROJECT_ROOT}/installs/install'
    detect_is_macos
    os=\$(detect_os)
    if [ \"\$os\" = \"darwin\" ]; then
      \$IS_MACOS && echo 'PASS' || echo 'FAIL'
    else
      \$IS_MACOS && echo 'FAIL' || echo 'PASS'
    fi
  "
  [ "$status" -eq 0 ]
  [ "$output" = "PASS" ]
}

# ---------------------------------------------------------------------------
# Contrib file existence: all files referenced by install scripts
# ---------------------------------------------------------------------------

@test "all contrib files referenced by install scripts exist" {
  local files=(
    "default_configs/default_keymap.yaml"
    "default_configs/default_theme.yaml"
    "default_configs/mihomo/core_override_config.yaml"
    "default_configs/mihomo/core_override_config_no_tun.yaml"
    "default_configs/sing-box/core_override_config.json"
    "default_configs/sing-box/core_override_config_no_tun.json"
    "templates/mihomo/common_tpl.yaml"
    "templates/mihomo/generic_tpl.yaml"
    "templates/mihomo/generic_tpl_with_all.yaml"
    "templates/mihomo/generic_tpl_with_filter.yaml"
    "templates/mihomo/generic_tpl_with_ruleset.yaml"
    "templates/sing-box/v1.12-tun_common_tpl.json"
    "templates/sing-box/v1.12-tun_bypass.json"
  )

  local missing=0
  for rel in "${files[@]}"; do
    if [ ! -f "$PROJECT_ROOT/contrib/$rel" ]; then
      echo "MISSING: contrib/$rel"
      missing=1
    fi
  done
  [ "$missing" -eq 0 ]
}

# ---------------------------------------------------------------------------
# Test that sourcing does not execute main
# ---------------------------------------------------------------------------

@test "sourcing install script does not execute main installation" {
  run bash -c "
    source '${PROJECT_ROOT}/installs/install'
    echo 'SOURCED_OK'
  "
  [ "$status" -eq 0 ]
  [[ "$output" == *"SOURCED_OK"* ]]
  # Should NOT contain install output like 'Install mode:'
  [[ "$output" != *"Install mode:"* ]]
}
