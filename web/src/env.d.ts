/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  // biome-ignore lint/suspicious/noExplicitAny: vue sfc module shim
  const component: DefineComponent<Record<string, never>, Record<string, never>, any>
  export default component
}
