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
    # Include _-prefixed names for escaped reserved words (e.g., _class, _delete)
    (grep -oE '__bg\._?[a-z][a-z0-9_]*\(' "$ts_file" || true) | sed 's/__bg\.//;s/($//' | sort -u | while read -r fn; do
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

# Create a stub for the FFI-mode uniffi_runtime.js import
cat > "$TMPDIR/uniffi_runtime.d.ts" <<'RUNTIME_STUB'
// Auto-generated stub for uniffi_runtime.js (FFI mode)
export declare class UniffiRuntime {
  static load(wasmUrl: URL, namespace: string): Promise<UniffiRuntime>;
  call(name: string, argPtr: number, retPtr: number): void;
  callFree(name: string, handle: bigint): void;
  getExport(name: string): Function;
  scratchAlloc(bytes: number): number;
  scratchReset(): void;
  scratchSave(): number;
  scratchRestore(offset: number): void;
  writeU8Element(ptr: number, v: number): void;
  writeI8Element(ptr: number, v: number): void;
  writeU16Element(ptr: number, v: number): void;
  writeI16Element(ptr: number, v: number): void;
  writeU32Element(ptr: number, v: number): void;
  writeI32Element(ptr: number, v: number): void;
  writeU64Element(ptr: number, v: bigint): void;
  writeI64Element(ptr: number, v: bigint): void;
  writeF32Element(ptr: number, v: number): void;
  writeF64Element(ptr: number, v: number): void;
  writeBoolElement(ptr: number, v: boolean): void;
  writeHandleElement(ptr: number, v: bigint): void;
  writeRustBufferElements(ptr: number, rb: any): void;
  readU8Element(ptr: number): number;
  readI8Element(ptr: number): number;
  readU16Element(ptr: number): number;
  readI16Element(ptr: number): number;
  readU32Element(ptr: number): number;
  readI32Element(ptr: number): number;
  readU64Element(ptr: number): bigint;
  readI64Element(ptr: number): bigint;
  readF32Element(ptr: number): number;
  readF64Element(ptr: number): number;
  readBoolElement(ptr: number): boolean;
  readHandleElement(ptr: number): bigint;
  readRustBufferElements(ptr: number): any;
  lowerString(s: string): any;
  liftString(rb: any): string;
  lowerBytes(b: Uint8Array): any;
  liftBytes(rb: any): Uint8Array;
  lowerIntoBuffer(fn: (w: UniFFIWriter) => void): any;
  liftFromBuffer<T>(rb: any, fn: (r: UniFFIReader) => T): T;
  checkCallStatus(ptr: number, liftError?: (rb: any) => Error): void;
  cloneObjectHandle(cloneFn: string, handle: bigint): bigint;
  insertCallbackHandle(obj: any): bigint;
  getCallbackHandle(handle: bigint): any;
  removeCallbackHandle(handle: bigint): void;
  cloneCallbackHandle(handle: bigint): bigint;
  pollToReady(futureHandle: bigint, pollFn: string): Promise<void>;
  _dv(): DataView;
  _readUtf8(ptr: number, len: number): string;
  _writeRustCallStatusStruct(ptr: number): void;
  _readRustBufferStruct(ptr: number): any;
  _writeRustBufferStruct(ptr: number, rb: any): void;
  _writeCallStatusSuccess(ptr: number): void;
  _writeCallStatusPanic(ptr: number, error: unknown): void;
  registerCallbackVTable(name: string, initFnName: string, entries: any[]): void;
}
export declare class UniFFIWriter {
  writeI8(v: number): void;
  writeU8(v: number): void;
  writeI16(v: number): void;
  writeU16(v: number): void;
  writeI32(v: number): void;
  writeU32(v: number): void;
  writeI64(v: bigint): void;
  writeU64(v: bigint): void;
  writeF32(v: number): void;
  writeF64(v: number): void;
  writeBool(v: boolean): void;
  writeString(v: string): void;
  writeBytes(v: Uint8Array): void;
  writeDuration(v: number): void;
  writeTimestamp(v: Date): void;
  writeOptional<T>(v: T | null | undefined, fn: (w: UniFFIWriter, v: T) => void): void;
  writeSequence<T>(v: T[], fn: (w: UniFFIWriter, v: T) => void): void;
  writeMap<K, V>(v: Map<K, V>, fk: (w: UniFFIWriter, k: K) => void, fv: (w: UniFFIWriter, v: V) => void): void;
}
export declare class UniFFIReader {
  readI8(): number;
  readU8(): number;
  readI16(): number;
  readU16(): number;
  readI32(): number;
  readU32(): number;
  readI64(): bigint;
  readU64(): bigint;
  readF32(): number;
  readF64(): number;
  readBool(): boolean;
  readString(): string;
  readBytes(): Uint8Array;
  readDuration(): number;
  readTimestamp(): Date;
  readOptional<T>(fn: (r: UniFFIReader) => T): T | null;
  readSequence<T>(fn: (r: UniFFIReader) => T): T[];
  readMap<K, V>(fk: (r: UniFFIReader) => K, fv: (r: UniFFIReader) => V): Map<K, V>;
}
RUNTIME_STUB

# Create tsconfig in the temp directory
cat > "$TMPDIR/tsconfig.json" <<'EOF'
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "lib": ["ES2022", "ESNext.Disposable", "DOM"],
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
tsc --noEmit 2>&1 || {
  echo ""
  echo "ERROR: Generated TypeScript files have type errors."
  echo "Fix the generator output and re-run."
  exit 1
}

echo "All golden files typecheck successfully."
