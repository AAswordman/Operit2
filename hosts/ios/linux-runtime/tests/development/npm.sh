set -eu
export NODE_OPTIONS=--jitless
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
mkdir package
cat > package/package.json <<'JSON'
{"name":"operit-runtime-fixture","version":"1.0.0","main":"index.cjs"}
JSON
printf 'module.exports = 42;\n' > package/index.cjs
cat > package.json <<'JSON'
{"name":"operit-runtime-consumer","version":"1.0.0","private":true,"scripts":{"verify":"node -e \"require('node:assert/strict').equal(require('operit-runtime-fixture'),42); console.log('NPM_INSTALL_SCRIPT_OK')\""}}
JSON
npm install --offline --no-audit --no-fund ./package
npm run verify
