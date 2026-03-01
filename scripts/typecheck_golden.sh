#!/usr/bin/env bash
# Typecheck all golden expected/*.ts files to verify they are valid TypeScript.
#
# Creates temporary .d.ts stub files for the wasm-bindgen imports, runs tsc,
# then cleans up. Any TypeScript structural errors in generated code will be
# caught here.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

# Collect all expected .ts files and create stubs
for ts_file in "$REPO_ROOT"/fixtures/*/expected/*.ts; do
  [ -f "$ts_file" ] || continue
  base="$(basename "$ts_file" .ts)"
  cp "$ts_file" "$TMPDIR/$base.ts"

  # Generate a .d.ts stub for the _bg.js import.
  # Scan the .ts file for __bg.ClassName and __bg.func_name references,
  # then declare matching exports so tsc can resolve them.
  {
    echo "// Auto-generated stub for ${base}_bg.js"
    echo "declare function init(...args: any[]): Promise<void>;"
    echo "export default init;"

    # Extract class names used as __bg.ClassName (type references and constructors)
    # || true to avoid pipefail on no matches
    (grep -oE '__bg\.[A-Z][A-Za-z0-9]*' "$ts_file" || true) | sed 's/__bg\.//' | sort -u | while read -r cls; do
      [ -z "$cls" ] && continue
      echo "export declare class $cls { [key: string]: any; constructor(...args: any[]); free(): void; }"
    done

    # Extract function names used as __bg.func_name( (function calls)
    (grep -oE '__bg\.[a-z][a-z0-9_]*\(' "$ts_file" || true) | sed 's/__bg\.//;s/($//' | sort -u | while read -r fn; do
      [ -z "$fn" ] && continue
      echo "export declare function $fn(...args: any[]): any;"
    done
  } > "$TMPDIR/${base}_bg.d.ts"

  # Handle any extra imports (like ext_types_demo's other_bindings.js)
  extra_imports=$( (grep -oE "from '\./[^']+\.js'" "$ts_file" || true) | (grep -v '_bg\.js' || true) | sed "s/from '\.\/\(.*\)\.js'/\1/" | sort -u)
  if [ -n "$extra_imports" ]; then
    echo "$extra_imports" | while read -r mod; do
      [ -z "$mod" ] && continue
      if [ ! -f "$TMPDIR/${mod}.d.ts" ]; then
        # Extract imported names for this module
        imported=$( (grep -oE "import \{ [^}]+ \} from '\.\/${mod}\.js'" "$ts_file" || true) | sed "s/import { //;s/ } from.*//")
        {
          echo "// Auto-generated stub for ${mod}.js"
          echo "$imported" | tr ',' '\n' | tr -d ' ' | while read -r name; do
            [ -z "$name" ] && continue
            echo "export interface $name { [key: string]: any; }"
          done
        } > "$TMPDIR/${mod}.d.ts"
      fi
    done
  fi
done

# Create tsconfig in the temp directory
cat > "$TMPDIR/tsconfig.json" <<'EOF'
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "lib": ["ES2022", "ESNext.Disposable"],
    "moduleResolution": "bundler",
    "strict": false,
    "noEmit": true,
    "skipLibCheck": true,
    "types": []
  },
  "include": ["*.ts"]
}
EOF

count=$(ls "$TMPDIR"/*.ts 2>/dev/null | grep -cv '\.d\.ts$' || true)
echo "Typechecking $count golden files..."
cd "$TMPDIR"

# Run tsc — if it fails, the generated TypeScript has structural errors.
npx tsc --noEmit 2>&1 || {
  echo ""
  echo "ERROR: Generated TypeScript files have type errors."
  echo "Fix the generator output and re-run."
  exit 1
}

echo "All golden files typecheck successfully."
