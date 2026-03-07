// Wildcard module declaration: any `.js` import resolves to a module with
// arbitrary named exports. This lets tsc validate the *structure* of the
// generated TypeScript (class declarations, export shapes, generic syntax)
// without needing real runtime modules.
declare module '*.js' {
  const init: (...args: any[]) => Promise<void>;
  export default init;
  export const __wasm: any;
  // Allow any named export via namespace index signature.
}
