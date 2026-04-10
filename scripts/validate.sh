#!/usr/bin/env bash

set -euo pipefail

TREE_SITTER_BIN="${TREE_SITTER_BIN:-tree-sitter}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PARSER_PARENT_DIR="$(cd "${REPO_ROOT}/.." && pwd)"
WORK_DIR="$(mktemp -d /tmp/tree-sitter-wpl-validate.XXXXXX)"
CONFIG_PATH="${WORK_DIR}/config.json"
HOME_DIR="${WORK_DIR}/home"

cleanup() {
  rm -rf "${WORK_DIR}"
}
trap cleanup EXIT

mkdir -p "${HOME_DIR}"

if ! command -v "${TREE_SITTER_BIN}" >/dev/null 2>&1; then
  echo "missing tree-sitter CLI: ${TREE_SITTER_BIN}" >&2
  exit 1
fi

cat > "${CONFIG_PATH}" <<EOF
{
  "parser-directories": [
    "${PARSER_PARENT_DIR}"
  ]
}
EOF

run_parse_check() {
  local name="$1"
  local file_path="${WORK_DIR}/${name}.wpl"

  cat > "${file_path}"
  echo "[parse] ${name}"
  HOME="${HOME_DIR}" "${TREE_SITTER_BIN}" parse --config-path "${CONFIG_PATH}" -q "${file_path}"
}

echo "[generate] grammar"
(
  cd "${REPO_ROOT}"
  "${TREE_SITTER_BIN}" generate
)

run_parse_check package <<'EOF'
package demo {
  #[tag(env: "prod"), copy_raw(name: "raw")]
  rule /svc/test {
    |json_like|strip/bom|
    (bad_json:raw)
  }
}
EOF

run_parse_check preproc <<'EOF'
rule demo {
  |plg_pipe(dayu)|decode/base64|strip/bom|
  (chars:payload)
}
EOF

run_parse_check subfields <<'EOF'
rule demo {
  (
    json(chars@name, opt(digit)@age:age_alias, @'@special-field', :fallback)
  )
}
EOF

run_parse_check pipes <<'EOF'
rule demo {
  (
    json(chars@name, digit@code)|f_has(name)|take("name")|regex_match('^adm')|not(chars_has(root)),
    chars:line|starts_with('/api/')
  )\,
}
EOF

echo "[highlight] package"
HOME="${HOME_DIR}" "${TREE_SITTER_BIN}" highlight \
  --config-path "${CONFIG_PATH}" \
  "${WORK_DIR}/package.wpl" >/dev/null

if [ -d "${REPO_ROOT}/test/corpus" ] || [ -d "${REPO_ROOT}/corpus" ]; then
  echo "[test] corpus"
  (
    cd "${REPO_ROOT}"
    HOME="${HOME_DIR}" "${TREE_SITTER_BIN}" test --config-path "${CONFIG_PATH}"
  )
fi

echo "tree-sitter-wpl validation passed"
