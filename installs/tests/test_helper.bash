# Shared helpers for install script bats tests

setup() {
  # Load common vars from install script in test mode
  REPO="JohanChane/clashtui"
  BRANCH="main"
  CORE_TYPE="all"
  IS_USER=false
  IS_TEST=true
  TEST_TMPDIR="$BATS_TEST_TMPDIR"
  MIHOMO_UPSTREAM="MetaCubeX/mihomo"
  SINGBOX_UPSTREAM="SagerNet/sing-box"

  # Create a temp dir simulating the project root (for local contrib discovery)
  PROJECT_DIR="$BATS_TEST_TMPDIR/project"
  mkdir -p "$PROJECT_DIR/contrib/config"
}

teardown() {
  true
}

# Mock uname to control OS and arch detection
mock_uname() {
  local mock_output="$1"
  function uname() {
    if [[ "$1" == "-m" ]]; then
      echo "${UNAME_M:-x86_64}"
    elif [[ "$1" == "-s" ]]; then
      echo "${UNAME_S:-Linux}"
    else
      command uname "$@"
    fi
  }
  export -f uname
}

mock_grep() {
  function grep() {
    if [[ "$*" == *"flags"*"/proc/cpuinfo"* ]]; then
      echo "${CPU_FLAGS:-flags : fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx lm}"
    else
      command grep "$@"
    fi
  }
  export -f grep
}

mock_curl() {
  function curl() {
    echo "mock curl: $*" >&2
    return 0
  }
  export -f curl
}

mock_command() {
  local name="$1"
  function "$name"() {
    echo "mocked $name"
  }
  export -f "$name"
}
