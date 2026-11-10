/**
 * Aphrody Store Web Standards Browser Client
 * Talks to /api/store/* endpoints with graceful offline fallback
 * Zero external npm dependencies - 100% pure Web Standards
 */

import type { StorePackage, StoreGeneration, Ecosystem } from "./types";
import { BUN_CURATED_PACKAGES } from "./drivers/bun";
import { WINGET_CURATED_PACKAGES } from "./drivers/winget";
import { WASM_HUB_PACKAGES } from "./drivers/wasm-hub";
import { NIX_CURATED_PACKAGES } from "./drivers/nix";
import { FLATPAK_CURATED_PACKAGES } from "./drivers/flatpak";
import { AUR_CURATED_PACKAGES } from "./drivers/aur";
import { PLAYSTORE_CURATED_PACKAGES } from "./drivers/playstore";

const ALL_CURATED: StorePackage[] = [
  ...BUN_CURATED_PACKAGES,
  ...WINGET_CURATED_PACKAGES,
  ...WASM_HUB_PACKAGES,
  ...NIX_CURATED_PACKAGES,
  ...FLATPAK_CURATED_PACKAGES,
  ...AUR_CURATED_PACKAGES,
  ...PLAYSTORE_CURATED_PACKAGES,
];

export class StoreClient {
  public async search(
    query = "",
    ecosystem: Ecosystem | "all" = "all",
  ): Promise<StorePackage[]> {
    try {
      const url = `/api/store/search?q=${encodeURIComponent(query)}&ecosystem=${ecosystem}`;
      const res = await fetch(url);
      if (res.ok) {
        const data = (await res.json()) as any;
        if (data && Array.isArray(data.packages)) {
          return data.packages;
        }
      }
    } catch (e) {
      // Offline fallback
    }

    const q = query.toLowerCase().trim();
    return ALL_CURATED.filter((p) => {
      const matchEco = ecosystem === "all" || p.ecosystem === ecosystem;
      const matchQ =
        !q ||
        p.name.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q);
      return matchEco && matchQ;
    });
  }

  public async installPackage(packageId: string): Promise<StoreGeneration> {
    try {
      const res = await fetch("/api/store/install", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ packageId }),
      });
      if (res.ok) {
        const data = (await res.json()) as any;
        return data.generation;
      }
    } catch (e) {
      // Offline fallback
    }

    return {
      id: Date.now(),
      timestamp: new Date().toISOString(),
      active: true,
      packages: [packageId],
      description: `Install ${packageId} (Client Session)`,
    };
  }

  public async listGenerations(): Promise<{
    active: StoreGeneration;
    generations: StoreGeneration[];
  }> {
    try {
      const res = await fetch("/api/store/generations");
      if (res.ok) {
        return (await res.json()) as any;
      }
    } catch (e) {
      // Offline fallback
    }

    const defaultGen: StoreGeneration = {
      id: 1,
      timestamp: new Date().toISOString(),
      active: true,
      packages: ["bun-elysia", "wasm-artcraft"],
      description: "Default System Baseline",
    };

    return {
      active: defaultGen,
      generations: [defaultGen],
    };
  }

  public async rollback(rollbackId: number): Promise<StoreGeneration> {
    try {
      const res = await fetch("/api/store/generations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ rollbackId }),
      });
      if (res.ok) {
        const data = (await res.json()) as any;
        return data.generation;
      }
    } catch (e) {
      // Offline fallback
    }

    return {
      id: Date.now(),
      timestamp: new Date().toISOString(),
      active: true,
      packages: [],
      description: `Rollback to Generation #${rollbackId}`,
      rollbackId,
    };
  }

  public async convertAwesome(markdownUrl: string): Promise<StorePackage[]> {
    try {
      const res = await fetch("/api/store/awesome-convert", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ markdownUrl }),
      });
      if (res.ok) {
        const data = (await res.json()) as any;
        return data.packages || [];
      }
    } catch (e) {
      // Offline fallback
    }

    return ALL_CURATED.slice(0, 4);
  }
}
