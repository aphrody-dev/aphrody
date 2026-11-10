/**
 * Aphrody Store SQLite Declarative State Engine
 * NixOS-style generations, rollbacks & package indexing
 * Built on pure bun:sqlite with zero external npm dependencies
 */

import { Database } from "bun:sqlite";
import { existsSync, mkdirSync } from "fs";
import { join } from "path";
import type { StorePackage, StoreGeneration } from "./types";

export class StoreDatabase {
  private db: Database;

  constructor(dbPath?: string) {
    if (dbPath && dbPath !== ":memory:") {
      const dir = join(dbPath, "..");
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
      }
      this.db = new Database(dbPath);
    } else {
      this.db = new Database(":memory:");
    }
    this.initSchema();
  }

  private initSchema(): void {
    this.db.run("PRAGMA synchronous = NORMAL;");
    this.db.run("PRAGMA journal_mode = WAL;");

    // 1. Packages Table
    this.db.run(`
      CREATE TABLE IF NOT EXISTS packages (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        ecosystem TEXT NOT NULL,
        version TEXT NOT NULL,
        description TEXT,
        author TEXT,
        repo_url TEXT,
        category TEXT,
        stars INTEGER DEFAULT 0,
        install_cmd TEXT,
        run_cmd TEXT,
        sandboxed INTEGER DEFAULT 0,
        tags TEXT,
        created_at TEXT
      );
    `);

    // 2. NixOS-Style Declarative Generations Table
    this.db.run(`
      CREATE TABLE IF NOT EXISTS generations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp TEXT NOT NULL,
        active INTEGER NOT NULL DEFAULT 0,
        packages_json TEXT NOT NULL,
        description TEXT NOT NULL,
        rollback_id INTEGER
      );
    `);

    // 3. Fast lookup indices
    this.db.run(`CREATE INDEX IF NOT EXISTS idx_pkg_name ON packages(name);`);
    this.db.run(
      `CREATE INDEX IF NOT EXISTS idx_pkg_eco ON packages(ecosystem);`,
    );
    this.db.run(
      `CREATE INDEX IF NOT EXISTS idx_pkg_cat ON packages(category);`,
    );

    // Ensure baseline Generation 1 exists
    const genCount = this.db
      .query("SELECT COUNT(*) as count FROM generations")
      .get() as { count: number };
    if (genCount.count === 0) {
      this.db.run(
        `
        INSERT INTO generations (timestamp, active, packages_json, description, rollback_id)
        VALUES ($ts, 1, '[]', 'Generation 1: Initial Clean State', NULL);
      `,
        { $ts: new Date().toISOString() },
      );
    }
  }

  public upsertPackage(pkg: StorePackage): void {
    const stmt = this.db.prepare(`
      INSERT INTO packages (
        id, name, ecosystem, version, description, author,
        repo_url, category, stars, install_cmd, run_cmd, sandboxed, tags, created_at
      ) VALUES (
        $id, $name, $ecosystem, $version, $description, $author,
        $repo_url, $category, $stars, $install_cmd, $run_cmd, $sandboxed, $tags, $created_at
      )
      ON CONFLICT(id) DO UPDATE SET
        version = excluded.version,
        description = excluded.description,
        stars = excluded.stars,
        install_cmd = excluded.install_cmd,
        run_cmd = excluded.run_cmd,
        sandboxed = excluded.sandboxed,
        tags = excluded.tags;
    `);

    stmt.run({
      $id: pkg.id,
      $name: pkg.name,
      $ecosystem: pkg.ecosystem,
      $version: pkg.version || "1.0.0",
      $description: pkg.description || "",
      $author: pkg.author || "",
      $repo_url: pkg.repoUrl || "",
      $category: pkg.category || "General",
      $stars: pkg.stars || 0,
      $install_cmd: pkg.installCmd || "",
      $run_cmd: pkg.runCmd || "",
      $sandboxed: pkg.sandboxed ? 1 : 0,
      $tags: JSON.stringify(pkg.tags || []),
      $created_at: new Date().toISOString(),
    });
  }

  public upsertBatch(packages: StorePackage[]): void {
    const insertTransaction = this.db.transaction((pkgs: StorePackage[]) => {
      for (const p of pkgs) {
        this.upsertPackage(p);
      }
    });
    insertTransaction(packages);
  }

  public searchPackages(
    query: string,
    ecosystem?: string,
    limit = 50,
  ): StorePackage[] {
    let sql = "SELECT * FROM packages WHERE 1=1";
    const params: Record<string, any> = { $limit: limit };

    if (ecosystem && ecosystem !== "all") {
      sql += " AND ecosystem = $ecosystem";
      params.$ecosystem = ecosystem;
    }

    if (query && query.trim() !== "") {
      sql += " AND (name LIKE $q OR description LIKE $q OR category LIKE $q)";
      params.$q = `%${query.trim()}%`;
    }

    sql += " ORDER BY stars DESC, name ASC LIMIT $limit";

    const rows = this.db.query(sql).all(params) as any[];
    return rows.map((r) => this.rowToPackage(r));
  }

  public getPackageById(id: string): StorePackage | null {
    const row = this.db
      .query("SELECT * FROM packages WHERE id = $id")
      .get({ $id: id }) as any;
    return row ? this.rowToPackage(row) : null;
  }

  private rowToPackage(r: any): StorePackage {
    return {
      id: r.id,
      name: r.name,
      ecosystem: r.ecosystem,
      version: r.version,
      description: r.description,
      author: r.author,
      repoUrl: r.repo_url,
      category: r.category,
      stars: r.stars,
      installCmd: r.install_cmd,
      runCmd: r.run_cmd,
      sandboxed: Boolean(r.sandboxed),
      tags: r.tags ? JSON.parse(r.tags) : [],
    };
  }

  // --- NixOS-Style Declarative Generations ---

  public getActiveGeneration(): StoreGeneration {
    const row = this.db
      .query(
        "SELECT * FROM generations WHERE active = 1 ORDER BY id DESC LIMIT 1",
      )
      .get() as any;
    if (!row) {
      return {
        id: 1,
        timestamp: new Date().toISOString(),
        active: true,
        packages: [],
        description: "Clean Baseline",
      };
    }
    return {
      id: row.id,
      timestamp: row.timestamp,
      active: true,
      packages: JSON.parse(row.packages_json),
      description: row.description,
      rollbackId: row.rollback_id || undefined,
    };
  }

  public listGenerations(): StoreGeneration[] {
    const rows = this.db
      .query("SELECT * FROM generations ORDER BY id DESC")
      .all() as any[];
    return rows.map((r) => ({
      id: r.id,
      timestamp: r.timestamp,
      active: Boolean(r.active),
      packages: JSON.parse(r.packages_json),
      description: r.description,
      rollbackId: r.rollback_id || undefined,
    }));
  }

  public createGeneration(
    description: string,
    packages: string[],
  ): StoreGeneration {
    this.db.run("UPDATE generations SET active = 0 WHERE active = 1;");
    const stmt = this.db.prepare(`
      INSERT INTO generations (timestamp, active, packages_json, description, rollback_id)
      VALUES ($ts, 1, $pkgs, $desc, NULL);
    `);
    const ts = new Date().toISOString();
    const result = stmt.run({
      $ts: ts,
      $pkgs: JSON.stringify(packages),
      $desc: description,
    });

    const newId = Number(result.lastInsertRowid);
    return {
      id: newId,
      timestamp: ts,
      active: true,
      packages,
      description,
    };
  }

  public rollbackToGeneration(targetId: number): StoreGeneration {
    const target = this.db
      .query("SELECT * FROM generations WHERE id = $id")
      .get({ $id: targetId }) as any;
    if (!target) {
      throw new Error(`Generation #${targetId} not found for rollback`);
    }

    const targetPackages = JSON.parse(target.packages_json);
    const desc = `Rollback to Generation #${targetId} (${target.description})`;

    this.db.run("UPDATE generations SET active = 0 WHERE active = 1;");
    const stmt = this.db.prepare(`
      INSERT INTO generations (timestamp, active, packages_json, description, rollback_id)
      VALUES ($ts, 1, $pkgs, $desc, $rbId);
    `);
    const ts = new Date().toISOString();
    const result = stmt.run({
      $ts: ts,
      $pkgs: JSON.stringify(targetPackages),
      $desc: desc,
      $rbId: targetId,
    });

    const newId = Number(result.lastInsertRowid);
    return {
      id: newId,
      timestamp: ts,
      active: true,
      packages: targetPackages,
      description: desc,
      rollbackId: targetId,
    };
  }

  public close(): void {
    this.db.close();
  }
}
