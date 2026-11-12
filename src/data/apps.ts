// SPDX-License-Identifier: Apache-2.0
/**
 * Aphrody Sovereign Suite App Registry
 * Replaces external third-party redirects with real internal Bun + WASM + SQLite applications:
 * Craft Creative Cloud, Fluent UI 2 Office 365, Spacedrive VDFS, M3 Workspace, and Local Studio.
 */

export interface GoogleApp {
  id: string;
  name: string;
  url: string;
  iconType: string;
}

export const SOVEREIGN_WORKSPACE_APPS: GoogleApp[] = [
  {
    id: "search",
    name: "Search 3-VPS",
    url: "/?suite=search",
    iconType: "search",
  },
  {
    id: "craft",
    name: "Craft Cloud",
    url: "/?suite=craft",
    iconType: "storytold",
  },
  {
    id: "docs",
    name: "WordCraft",
    url: "/?suite=docs",
    iconType: "docs",
  },
  {
    id: "sheets",
    name: "GridCraft",
    url: "/?suite=sheets",
    iconType: "sheets",
  },
  {
    id: "slides",
    name: "DeckCraft",
    url: "/?suite=slides",
    iconType: "slides",
  },
  {
    id: "drive",
    name: "Spacedrive",
    url: "/?suite=drive",
    iconType: "drive",
  },
  {
    id: "mail",
    name: "Aphrody Mail",
    url: "/?suite=mail",
    iconType: "gmail",
  },
  {
    id: "notes",
    name: "M3 Notes",
    url: "/?suite=notes",
    iconType: "keep",
  },
  {
    id: "calendar",
    name: "Agenda .ics",
    url: "/?suite=calendar",
    iconType: "calendar",
  },
  {
    id: "contacts",
    name: "Contacts .vcf",
    url: "/?suite=contacts",
    iconType: "contacts",
  },
  {
    id: "translate",
    name: "Traduction",
    url: "/?suite=translate",
    iconType: "translate",
  },
  {
    id: "photos",
    name: "PhotoCraft",
    url: "/?suite=photos",
    iconType: "photos",
  },
  {
    id: "studio",
    name: "Local Studio",
    url: "/?suite=studio",
    iconType: "chat",
  },
  {
    id: "account",
    name: "Compte Argon2",
    url: "/?suite=account",
    iconType: "account",
  },
];

// Preserved alias for existing module consumers
export const GOOGLE_APPS: GoogleApp[] = SOVEREIGN_WORKSPACE_APPS;

export const APHRODY_SYSTEM_APPS: GoogleApp[] = [
  {
    id: "storytold",
    name: "Storytold 3D & Shaders",
    url: "#storytold",
    iconType: "storytold",
  },
  { id: "store", name: "Universal Store", url: "#store", iconType: "store" },
  { id: "bench", name: "Stack Benchmark", url: "#bench", iconType: "bench" },
  {
    id: "shell",
    name: "Mode Shell Adaptatif",
    url: "#shell",
    iconType: "shell",
  },
];

export const ALL_APPS: GoogleApp[] = [
  ...SOVEREIGN_WORKSPACE_APPS,
  ...APHRODY_SYSTEM_APPS,
];

function wrapSvg(body: string): string {
  return `<svg class="app-svg" viewBox="0 0 40 40" aria-hidden="true">${body}</svg>`;
}

export function getAppIconSvg(type: string): string {
  switch (type) {
    case "account":
      return wrapSvg(
        '<circle cx="20" cy="20" r="18" fill="#0b57d0"/><circle cx="20" cy="15" r="6" fill="#fff"/><path d="M10 32c0-5.5 4.5-10 10-10s10 4.5 10 10" fill="#fff"/>',
      );
    case "search":
      return wrapSvg(
        '<circle cx="20" cy="20" r="18" fill="#eef3fc" stroke="#c4c7c5"/><path d="M18 13a6 6 0 1 0 4.24 10.24l4.5 4.5 1.41-1.41-4.5-4.5A6 6 0 0 0 18 13z" fill="#0b57d0"/>',
      );
    case "gmail":
      return wrapSvg(
        '<rect x="8" y="11" width="24" height="18" rx="3" fill="#fff" stroke="#0b57d0" stroke-width="2"/><path d="M8 12l12 9 12-9" fill="none" stroke="#0b57d0" stroke-width="2.5"/>',
      );
    case "chat":
      return wrapSvg(
        '<path d="M8 12c0-2.2 1.8-4 4-4h16c2.2 0 4 1.8 4 4v12c0 2.2-1.8 4-4 4H14l-6 5V12z" fill="#0f6cbd"/><circle cx="15" cy="18" r="2" fill="#fff"/><circle cx="20" cy="18" r="2" fill="#fff"/><circle cx="25" cy="18" r="2" fill="#fff"/>',
      );
    case "drive":
      return wrapSvg(
        '<rect x="7" y="9" width="26" height="22" rx="5" fill="#0066da"/><circle cx="20" cy="20" r="6" fill="none" stroke="#fff" stroke-width="2.5"/><circle cx="20" cy="20" r="2" fill="#ffba00"/>',
      );
    case "calendar":
      return wrapSvg(
        '<rect x="8" y="9" width="24" height="22" rx="3" fill="#fff" stroke="#0b57d0" stroke-width="2"/><rect x="8" y="9" width="24" height="7" fill="#0b57d0"/><text x="20" y="26" font-size="11" font-weight="bold" fill="#0b57d0" text-anchor="middle">31</text>',
      );
    case "contacts":
      return wrapSvg(
        '<rect x="9" y="8" width="22" height="24" rx="3" fill="#0b57d0"/><circle cx="20" cy="16" r="4" fill="#fff"/><path d="M14 26c0-3.3 2.7-6 6-6s6 2.7 6 6" fill="#fff"/>',
      );
    case "translate":
      return wrapSvg(
        '<rect x="7" y="9" width="14" height="14" rx="3" fill="#0b57d0"/><rect x="19" y="17" width="14" height="14" rx="3" fill="#00832d"/><text x="14" y="19" font-size="9" fill="#fff" font-weight="bold" text-anchor="middle">FR</text><text x="26" y="27" font-size="9" fill="#fff" font-weight="bold" text-anchor="middle">EN</text>',
      );
    case "photos":
      return wrapSvg(
        '<rect x="7" y="9" width="26" height="22" rx="4" fill="#0b57d0"/><circle cx="15" cy="16" r="3" fill="#ffba00"/><polygon points="10,28 19,19 25,25 28,22 31,28" fill="#fff"/>',
      );
    case "docs":
      return wrapSvg(
        '<path d="M12 8h11l7 7v17a2 2 0 0 1-2 2H12a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z" fill="#0f6cbd"/><polygon points="23,8 30,15 23,15" fill="#a1c2fa"/><line x1="15" y1="20" x2="25" y2="20" stroke="#fff" stroke-width="2"/><line x1="15" y1="24" x2="25" y2="24" stroke="#fff" stroke-width="2"/>',
      );
    case "sheets":
      return wrapSvg(
        '<path d="M12 8h11l7 7v17a2 2 0 0 1-2 2H12a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z" fill="#0f9d58"/><polygon points="23,8 30,15 23,15" fill="#87ceac"/><line x1="15" y1="20" x2="25" y2="20" stroke="#fff" stroke-width="1.5"/><line x1="15" y1="24" x2="25" y2="24" stroke="#fff" stroke-width="1.5"/><line x1="20" y1="18" x2="20" y2="26" stroke="#fff" stroke-width="1.5"/>',
      );
    case "slides":
      return wrapSvg(
        '<path d="M12 8h11l7 7v17a2 2 0 0 1-2 2H12a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z" fill="#e37400"/><polygon points="23,8 30,15 23,15" fill="#fbe7a1"/><rect x="15" y="19" width="10" height="7" rx="1" fill="#fff"/>',
      );
    case "keep":
      return wrapSvg(
        '<rect x="8" y="8" width="24" height="24" rx="6" fill="#fbbc04"/><path d="M15 23h10v2H15z" fill="#fff"/><path d="M20 12a5 5 0 0 0-4 8h8a5 5 0 0 0-4-8z" fill="#fff"/>',
      );
    case "storytold":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#0b57d0"/><path d="M14 26l6-12 6 12-4-2-2 4-2-4z" fill="#ffffff"/><circle cx="20" cy="14" r="2.5" fill="#ffba00"/>',
      );
    case "store":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#0f9d58"/><path d="M10 14h20v4l-2 8H12l-2-8v-4z" fill="#ffffff"/><path d="M16 14v-2a4 4 0 0 1 8 0v2" fill="none" stroke="#ffffff" stroke-width="2"/>',
      );
    case "bench":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#c5221f"/><circle cx="20" cy="22" r="9" fill="none" stroke="#ffffff" stroke-width="2.5"/><line x1="20" y1="22" x2="25" y2="17" stroke="#ffffff" stroke-width="2" stroke-linecap="round"/>',
      );
    case "shell":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#0078d4"/><rect x="11" y="11" width="8" height="8" rx="1.5" fill="#ffffff"/><rect x="21" y="11" width="8" height="8" rx="1.5" fill="#ffffff"/><rect x="11" y="21" width="8" height="8" rx="1.5" fill="#ffffff"/><rect x="21" y="21" width="8" height="8" rx="1.5" fill="#ffffff"/>',
      );
    default:
      return wrapSvg(
        '<circle cx="20" cy="20" r="16" fill="#0b57d0"/><text x="20" y="25" font-size="14" fill="#fff" font-weight="bold" text-anchor="middle">A</text>',
      );
  }
}
