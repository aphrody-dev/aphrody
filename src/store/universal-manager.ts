/**
 * Aphrody Universal Package & Store Manager
 * Unified interface orchestrating all drivers, bun:sqlite state & NixOS-style generations
 * Zero external npm dependencies
 */

import { StoreDatabase } from "./database";
import { searchBunPackages, BUN_CURATED_PACKAGES } from "./drivers/bun";
import {
  searchWingetPackages,
  WINGET_CURATED_PACKAGES,
} from "./drivers/winget";
import { searchAurPackages, AUR_CURATED_PACKAGES } from "./drivers/aur";
import { searchNixPackages, NIX_CURATED_PACKAGES } from "./drivers/nix";
import {
  searchFlatpakPackages,
  FLATPAK_CURATED_PACKAGES,
} from "./drivers/flatpak";
import {
  searchPlayStorePackages,
  PLAYSTORE_CURATED_PACKAGES,
} from "./drivers/playstore";
import { searchWasmHubPackages, WASM_HUB_PACKAGES } from "./drivers/wasm-hub";
import { AwesomeListConverter } from "./drivers/awesome-converter";
import { pickOptimalStack } from "../benchmark/stack-picker";
import type { StorePackage, StoreGeneration, Ecosystem } from "./types";

export class UniversalStoreManager {
  private static instance: UniversalStoreManager;
  private db: StoreDatabase;

  constructor(dbPath?: string) {
    this.db = new StoreDatabase(dbPath);
    this.seedCuratedPackages();
  }

  public static getInstance(dbPath?: string): UniversalStoreManager {
    if (!UniversalStoreManager.instance) {
      UniversalStoreManager.instance = new UniversalStoreManager(dbPath);
    }
    return UniversalStoreManager.instance;
  }

  private seedCuratedPackages(): void {
    const all = [
      ...BUN_CURATED_PACKAGES,
      ...WINGET_CURATED_PACKAGES,
      ...AUR_CURATED_PACKAGES,
      ...NIX_CURATED_PACKAGES,
      ...FLATPAK_CURATED_PACKAGES,
      ...PLAYSTORE_CURATED_PACKAGES,
      ...WASM_HUB_PACKAGES,
    ];
    this.db.upsertBatch(all);
  }

  public async search(
    query: string,
    ecosystem: Ecosystem | "all" = "all",
  ): Promise<StorePackage[]> {
    // 1. Check local SQLite index first
    let localResults = this.db.searchPackages(query, ecosystem, 30);

    // If query has results in SQLite, enrich with picked stack and return
    if (localResults.length > 0) {
      return localResults.map((pkg) => ({
        ...pkg,
        pickedStack:
          pkg.pickedStack ||
          pickOptimalStack({ name: pkg.name, language: pkg.category }),
      }));
    }

    // 2. Query individual drivers dynamically
    let dynamicResults: StorePackage[] = [];
    if (ecosystem === "all" || ecosystem === "bun") {
      dynamicResults.push(...(await searchBunPackages(query)));
    }
    if (ecosystem === "all" || ecosystem === "winget") {
      dynamicResults.push(...(await searchWingetPackages(query)));
    }
    if (ecosystem === "all" || ecosystem === "aur") {
      dynamicResults.push(...(await searchAurPackages(query)));
    }
    if (ecosystem === "all" || ecosystem === "nix") {
      dynamicResults.push(...(await searchNixPackages(query)));
    }
    if (ecosystem === "all" || ecosystem === "flatpak") {
      dynamicResults.push(...(await searchFlatpakPackages(query)));
    }
    if (ecosystem === "all" || ecosystem === "playstore") {
      dynamicResults.push(...(await searchPlayStorePackages(query)));
    }
    if (ecosystem === "all" || ecosystem === "wasm-hub") {
      dynamicResults.push(...(await searchWasmHubPackages(query)));
    }

    // Index discovered packages into SQLite for fast subsequent lookups
    if (dynamicResults.length > 0) {
      this.db.upsertBatch(dynamicResults);
    }

    return dynamicResults.map((pkg) => ({
      ...pkg,
      pickedStack: pickOptimalStack({ name: pkg.name, language: pkg.category }),
    }));
  }

  public async convertAwesomeList(
    markdownOrUrl: string,
  ): Promise<StorePackage[]> {
    let result;
    if (
      markdownOrUrl.startsWith("http://") ||
      markdownOrUrl.startsWith("https://")
    ) {
      result = await AwesomeListConverter.fetchAndConvert(markdownOrUrl);
    } else {
      result = AwesomeListConverter.parseMarkdown(
        markdownOrUrl,
        "Custom Awesome List",
      );
    }

    if (result.packages.length > 0) {
      this.db.upsertBatch(result.packages);
    }

    return result.packages;
  }

  public installPackage(packageId: string): StoreGeneration {
    const pkg = this.db.getPackageById(packageId);
    if (!pkg) {
      throw new Error(`Package ${packageId} not found in Store`);
    }

    const currentGen = this.db.getActiveGeneration();
    const newPkgs = Array.from(new Set([...currentGen.packages, packageId]));
    return this.db.createGeneration(
      `Install ${pkg.name} (${pkg.ecosystem})`,
      newPkgs,
    );
  }

  public uninstallPackage(packageId: string): StoreGeneration {
    const currentGen = this.db.getActiveGeneration();
    const newPkgs = currentGen.packages.filter((id) => id !== packageId);
    return this.db.createGeneration(`Uninstall ${packageId}`, newPkgs);
  }

  public rollback(targetGenerationId: number): StoreGeneration {
    return this.db.rollbackToGeneration(targetGenerationId);
  }

  public listGenerations(): StoreGeneration[] {
    return this.db.listGenerations();
  }

  public getActiveGeneration(): StoreGeneration {
    return this.db.getActiveGeneration();
  }

  public getDatabase(): StoreDatabase {
    return this.db;
  }
}
