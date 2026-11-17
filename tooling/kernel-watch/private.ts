// Where generated output lives. Pages, records and probe results describe the owner's machines and the state of the work, so they are
// never tracked: they go to a git-ignored directory (KERNEL_WATCH_PRIVATE_DIR overrides it), like the private store of the other repositories.
import { join, resolve } from "node:path";

export const privateDir = (): string => resolve(Bun.env.KERNEL_WATCH_PRIVATE_DIR ?? join(import.meta.dir, "..", "..", ".private", "kernel-watch"));
export const defaultOutDir = (): string => join(privateDir(), "watch");
