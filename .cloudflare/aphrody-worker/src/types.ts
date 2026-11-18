export interface R2ObjectLike {
  body: ReadableStream | string | Uint8Array | null;
  httpEtag?: string;
  writeHttpMetadata?: (headers: Headers) => void;
  text?: () => Promise<string>;
  json?: <T = unknown>() => Promise<T>;
}

export interface R2ListedObjectLike {
  key: string;
  size?: number;
  etag?: string;
  uploaded?: Date | string;
}

export interface R2ObjectsLike {
  objects: R2ListedObjectLike[];
  truncated?: boolean;
  cursor?: string;
}

export interface R2BucketLike {
  get(key: string): Promise<R2ObjectLike | null>;
  put?(
    key: string,
    value: string | Uint8Array | ArrayBuffer | ReadableStream,
    options?: { httpMetadata?: { contentType?: string } },
  ): Promise<unknown>;
  list?(options?: { prefix?: string; limit?: number; cursor?: string }): Promise<R2ObjectsLike>;
  delete?(keys: string | string[]): Promise<void>;
}

export interface QueueLike<T = unknown> {
  send(message: T, options?: { contentType?: "json" | "text" | "bytes" | "v8" }): Promise<void>;
}

export interface ScheduledControllerLike {
  cron?: string;
  scheduledTime?: number;
}

export interface ExecutionContextLike {
  waitUntil?(promise: Promise<unknown>): void;
  passThroughOnException?(): void;
}

export interface WorkerEnv {
  RELEASES_BUCKET?: R2BucketLike;
  ASSETS_BUCKET?: R2BucketLike;
  OPEN_SOURCE_WEBSITE_ASSETS_BUCKET?: R2BucketLike;
  DOCS_ORIGIN_URL?: string;
  GITHUB_WEBHOOK_SECRET?: string;
  FLEET_BEACON_TOKEN?: string;
  LEARN_SYNC_TOKEN?: string;
  LEARN_PUBLIC_READ?: string;
  EVENTS?: QueueLike;
}

export const MONOGRAM_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96" role="img" aria-label="Aphrody Monogram"><title>Aphrody</title><rect width="96" height="96" rx="22" fill="#004B75"/><path d="M48 18 76 74H63.5L57.8 62H38.2L32.5 74H20L48 18Zm0 19-6.6 15.5h13.2L48 37Z" fill="#FFFFFF"/><path d="M56.5 18 42.5 47H54L39.5 76" fill="none" stroke="#FFB300" stroke-width="5.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
