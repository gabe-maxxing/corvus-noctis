#!/bin/sh
# Builds the Pages site into web/www (used by ci.yml and pages.yml).
#   .github/scripts/build-web.sh [VERSION]
# Needs: rust + wasm32-unknown-unknown, wasm-bindgen-cli =0.2.130, node/npx.
# Output must run on iOS 12 Safari (iPad Air 1): MVP WebAssembly only and
# ES2017 JavaScript. Both are checked here; the build fails otherwise.
set -eu
version="${1:-dev}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
www="$root/web/www"
tools="$root/web/target/node-tools"
BINARYEN=132.0.0
ACORN=8.19.0

if [ ! -x "$tools/node_modules/.bin/wasm-opt" ] || [ ! -x "$tools/node_modules/.bin/acorn" ]; then
  mkdir -p "$tools"
  (cd "$tools" && [ -f package.json ] || echo '{"private":true}' > "$tools/package.json")
  (cd "$tools" && npm install --silent --no-audit --no-fund "binaryen@$BINARYEN" "acorn@$ACORN")
fi
bin="$tools/node_modules/.bin"

echo "== wasm"
(cd "$root/web" && cargo build --release)
rm -rf "$www/app/pkg"
wasm-bindgen --target web --no-typescript --out-dir "$www/app/pkg" \
  "$root/web/target/wasm32-unknown-unknown/release/corvus_web.wasm"

# Rust's wasm32 std uses sign-ext and bulk-memory ops; Safari 12 only knows MVP.
wasm="$www/app/pkg/corvus_web_bg.wasm"
"$bin/wasm-opt" "$wasm" --enable-sign-ext --enable-bulk-memory --enable-bulk-memory-opt \
  --enable-nontrapping-float-to-int --enable-mutable-globals \
  --signext-lowering --llvm-memory-copy-fill-lowering --llvm-nontrapping-fptoint-lowering \
  --strip-target-features -Os -o "$wasm.mvp"
mv "$wasm.mvp" "$wasm"
if "$bin/wasm-dis" "$wasm" | grep -q -E 'extend(8|16|32)_s|memory\.(copy|fill|init)|data\.drop|trunc_sat|ref\.(null|func|is_null)|table\.(get|set|grow|size|fill)|\(result [a-z0-9]+ [a-z0-9]+'; then
  echo "::error::web wasm uses non-MVP features (breaks iOS 12 Safari)"; exit 1
fi
echo "wasm: MVP only, $(wc -c < "$wasm") bytes"

# Resolve the wasm path without import.meta so the glue parses as plain ES2017.
glue="$www/app/pkg/corvus_web.js"
sed -i.bak "s|new URL('corvus_web_bg.wasm', import.meta.url)|'pkg/corvus_web_bg.wasm'|" "$glue" && rm -f "$glue.bak"

echo "== site"
cp "$root/dist/install.sh" "$www/install.sh"
cp "$root/diogenes/DIOGENES.md" "$www/app/diogenes.md"
cp "$root/docs/brand/social-preview.png" "$www/social-preview.png"
cp "$root/docs/brand/corvus-logo.png" "$www/icons/corvus-logo.png"
sed "s|__VERSION__|$version|" "$www/sw.template.js" > "$www/sw.js"

echo "== ES2017 check (iOS 12)"
for js in "$glue" "$www/sw.js" "$www"/*.js "$www"/app/*.js; do
  [ -f "$js" ] || continue
  case "$js" in */sw.template.js) continue ;; esac
  kind=--module; case "$js" in */sw.js) kind="" ;; esac
  "$bin/acorn" --ecma2017 $kind --silent "$js" || { echo "::error::$js is newer than ES2017 (breaks iOS 12 Safari)"; exit 1; }
done
echo "site ready in web/www (version $version)"
