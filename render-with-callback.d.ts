import type { Environment } from './index.js'

export function renderWithCallback(
  env: Environment,
  name: string,
  ctx: unknown,
  cb: (err: Error | null, html?: string) => void,
): void

export function renderWithCallbackAsync(
  env: Environment,
  name: string,
  ctx: unknown,
  cb: (err: Error | null, html?: string) => void,
): void
