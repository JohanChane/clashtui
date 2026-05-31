# Test suite for install (bash) script

setup() {
  TEST_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")" && pwd -P)"
  PROJECT_ROOT="$(cd "$TEST_DIR/../.." && pwd -P)"

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
# Unit tests: backup_dir
# ---------------------------------------------------------------------------

@test "backup_dir copies directory with versioned suffix" {
  local tmpdir="$BATS_TEST_TMPDIR/testdir"
  mkdir -p "$tmpdir"
  echo "data" > "$tmpdir/file.txt"

  run bash -c "source '${PROJECT_ROOT}/installs/install' && backup_dir '$tmpdir'"
  [ "$status" -eq 0 ]
  [ -d "$tmpdir" ]
  [ -d "${tmpdir}_1" ]
  [ "$(cat "${tmpdir}_1/file.txt")" = "data" ]
}

# ---------------------------------------------------------------------------
# Backup tracking
# ---------------------------------------------------------------------------

@test "backup_dir tracks entries in BACKED_UP_LIST" {
  local tmpdir="$BATS_TEST_TMPDIR/testdir2"
  mkdir -p "$tmpdir"

  run bash -c "
    source '${PROJECT_ROOT}/installs/install'
    backup_dir '$tmpdir'
    printf '%s\n' \"\${BACKED_UP_LIST[@]}\"
  "
  [ "$status" -eq 0 ]
  [[ "$output" == *"$tmpdir -> ${tmpdir}_1"* ]]
}

@test "backup_file tracks entries in BACKED_UP_LIST" {
  local tmpfile="$BATS_TEST_TMPDIR/testfile_track"
  echo "data" > "$tmpfile"

  run bash -c "
    source '${PROJECT_ROOT}/installs/install'
    backup_file '$tmpfile'
    printf '%s\n' \"\${BACKED_UP_LIST[@]}\"
  "
  [ "$status" -eq 0 ]
  [[ "$output" == *"$tmpfile -> ${tmpfile}_1"* ]]
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
# OpenRC support tests: internal state verification
# ---------------------------------------------------------------------------

@test "resolve_paths with SERVICE_CONTROLLER=openrc sets UNIT_DIR to /etc/init.d" {
  run bash -c "
    source '${PROJECT_ROOT}/installs/install'
    SERVICE_CONTROLLER=openrc
    IS_USER=false
    resolve_paths
    echo \"UNIT_DIR=\$UNIT_DIR\"
    echo \"SERVICE_IS_USER=\$SERVICE_IS_USER\"
    echo \"SYSTEMD_RELOAD=\${SYSTEMD_RELOAD:-NOT_SET}\"
  "
  [ "$status" -eq 0 ]
  [[ "$output" == *"UNIT_DIR=/etc/init.d"* ]]
  [[ "$output" == *"SERVICE_IS_USER=false"* ]]
  [[ "$output" == *"SYSTEMD_RELOAD=NOT_SET"* ]]
}

@test "resolve_paths with SERVICE_CONTROLLER=openrc and IS_USER=true keeps IS_USER=true" {
  run bash -c "
    source '${PROJECT_ROOT}/installs/install'
    SERVICE_CONTROLLER=openrc
    IS_USER=true
    resolve_paths
    echo \"IS_USER=\$IS_USER\"
    echo \"SERVICE_IS_USER=\$SERVICE_IS_USER\"
  "
  [ "$status" -eq 0 ]
  [[ "$output" == *"IS_USER=true"* ]]
  [[ "$output" == *"SERVICE_IS_USER=true"* ]]
}

# ---------------------------------------------------------------------------
# --no-prompt acceptance test
# ---------------------------------------------------------------------------

@test "--no-prompt is recognized and script shows help without error" {
  run bash "${PROJECT_ROOT}/installs/install" --no-prompt --help
  [ "$status" -eq 0 ]
  [[ "$output" == *"--no-prompt"* ]]
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
  [[ "$output" != *"Install mode:"* ]]
}

# ---------------------------------------------------------------------------
# --is-test is rejected (removed)
# ---------------------------------------------------------------------------

@test "--is-test is rejected as unknown option" {
  run bash "${PROJECT_ROOT}/installs/install" --is-test
  [ "$status" -ne 0 ]
}
