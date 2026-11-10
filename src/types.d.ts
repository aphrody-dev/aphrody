/**
 * Native Bun, Node & Web Ambient Types
 * Strictly zero external npm dependencies!
 */

declare global {
  const Bun: any;
  const Buffer: any;
  const process: any;

  interface ImportMeta {
    dir: string;
    url: string;
  }
}

declare module "bun" {
  export const serve: any;
  export const gzipSync: any;
  export const nanoseconds: any;
  export const build: any;
}

declare module "bun:test" {
  export const describe: (name: string, fn: () => void | Promise<void>) => void;
  export const it: (name: string, fn: () => void | Promise<void>) => void;
  export const expect: (actual: any) => any;
  export const beforeEach: (fn: () => void | Promise<void>) => void;
  export const afterEach: (fn: () => void | Promise<void>) => void;
}

declare module "bun:sqlite" {
  export class Database {
    constructor(path?: string, options?: any);
    run(query: string, params?: any): any;
    query(query: string): any;
    prepare(query: string): any;
    transaction(fn: (...args: any[]) => any): any;
    close(): void;
  }
}

declare module "fs" {
  export function existsSync(path: string): boolean;
  export function readFileSync(path: string, encoding?: string): any;
  export function writeFileSync(
    path: string,
    data: any,
    encoding?: string,
  ): void;
  export function mkdirSync(path: string, options?: any): any;
  export function statSync(path: string): any;
}

declare module "path" {
  export function join(...paths: string[]): string;
  export function resolve(...paths: string[]): string;
}
