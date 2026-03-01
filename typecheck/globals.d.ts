// Wildcard module declaration: any `.js` import resolves to a module where
// the default export is an init function and all named exports are `any`.
// This lets tsc validate the *structure* of the generated TypeScript (class
// declarations, export shapes, generic syntax) without needing real wasm-bindgen
// glue files.
declare module '*.js' {
  const init: (...args: any[]) => Promise<void>;
  export default init;
  export const __wasm: any;
  // Allow any named export via namespace index signature.
}
