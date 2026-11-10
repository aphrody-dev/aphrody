export interface GoogleApp {
  id: string;
  name: string;
  url: string;
  iconType: string;
}

export const GOOGLE_APPS: GoogleApp[] = [
  {
    id: "account",
    name: "Compte",
    url: "https://myaccount.google.com/?hl=fr",
    iconType: "account",
  },
  {
    id: "search",
    name: "Recherche",
    url: "https://www.google.com/?hl=fr",
    iconType: "search",
  },
  {
    id: "maps",
    name: "Maps",
    url: "https://maps.google.com/?hl=fr",
    iconType: "maps",
  },
  {
    id: "youtube",
    name: "YouTube",
    url: "https://www.youtube.com/?hl=fr",
    iconType: "youtube",
  },
  {
    id: "news",
    name: "Actualités",
    url: "https://news.google.com/?hl=fr",
    iconType: "news",
  },
  {
    id: "gmail",
    name: "Gmail",
    url: "https://mail.google.com/",
    iconType: "gmail",
  },
  {
    id: "meet",
    name: "Meet",
    url: "https://meet.google.com/",
    iconType: "meet",
  },
  {
    id: "chat",
    name: "Chat",
    url: "https://chat.google.com/",
    iconType: "chat",
  },
  {
    id: "drive",
    name: "Drive",
    url: "https://drive.google.com/",
    iconType: "drive",
  },
  {
    id: "calendar",
    name: "Agenda",
    url: "https://calendar.google.com/",
    iconType: "calendar",
  },
  {
    id: "contacts",
    name: "Contacts",
    url: "https://contacts.google.com/",
    iconType: "contacts",
  },
  {
    id: "translate",
    name: "Traduction",
    url: "https://translate.google.com/?hl=fr",
    iconType: "translate",
  },
  {
    id: "photos",
    name: "Photos",
    url: "https://photos.google.com/",
    iconType: "photos",
  },
  {
    id: "shopping",
    name: "Shopping",
    url: "https://www.google.com/shopping?hl=fr",
    iconType: "shopping",
  },
  {
    id: "docs",
    name: "Docs",
    url: "https://docs.google.com/document/",
    iconType: "docs",
  },
  {
    id: "sheets",
    name: "Sheets",
    url: "https://docs.google.com/spreadsheets/",
    iconType: "sheets",
  },
  {
    id: "slides",
    name: "Slides",
    url: "https://docs.google.com/presentation/",
    iconType: "slides",
  },
  {
    id: "keep",
    name: "Keep",
    url: "https://keep.google.com/",
    iconType: "keep",
  },
];

export const APHRODY_SYSTEM_APPS: GoogleApp[] = [
  {
    id: "storytold",
    name: "Storytold Creative Suite",
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

export const ALL_APPS: GoogleApp[] = [...GOOGLE_APPS, ...APHRODY_SYSTEM_APPS];

function wrapSvg(body: string): string {
  return `<svg class="app-svg" viewBox="0 0 40 40">${body}</svg>`;
}

export function getAppIconSvg(type: string): string {
  switch (type) {
    case "account":
      return wrapSvg(
        '<circle cx="20" cy="20" r="18" fill="#1a73e8"/><circle cx="20" cy="15" r="6" fill="#fff"/><path d="M10 32c0-5.5 4.5-10 10-10s10 4.5 10 10" fill="#fff"/>',
      );
    case "search":
      return wrapSvg(
        '<circle cx="20" cy="20" r="18" fill="#f8f9fa" stroke="#dfe1e5"/><path d="M18 13a6 6 0 1 0 4.24 10.24l4.5 4.5 1.41-1.41-4.5-4.5A6 6 0 0 0 18 13z" fill="#4285f4"/>',
      );
    case "maps":
      return wrapSvg(
        '<path d="M20 6c-5.5 0-10 4.5-10 10 0 7.5 10 18 10 18s10-10.5 10-18c0-5.5-4.5-10-10-10z" fill="#ea4335"/><circle cx="20" cy="16" r="4" fill="#fff"/>',
      );
    case "youtube":
      return wrapSvg(
        '<rect x="6" y="10" width="28" height="20" rx="6" fill="#ff0000"/><polygon points="17,15 26,20 17,25" fill="#fff"/>',
      );
    case "news":
      return wrapSvg(
        '<rect x="8" y="10" width="24" height="20" rx="3" fill="#1a73e8"/><rect x="12" y="14" width="8" height="6" fill="#fff"/><line x1="22" y1="15" x2="28" y2="15" stroke="#fff" stroke-width="2"/><line x1="22" y1="19" x2="28" y2="19" stroke="#fff" stroke-width="2"/><line x1="12" y1="23" x2="28" y2="23" stroke="#fff" stroke-width="2"/><line x1="12" y1="26" x2="24" y2="26" stroke="#fff" stroke-width="2"/>',
      );
    case "gmail":
      return wrapSvg(
        '<rect x="8" y="11" width="24" height="18" rx="2" fill="#fff" stroke="#dadce0"/><path d="M8 11l12 9 12-9" fill="none" stroke="#ea4335" stroke-width="3"/><path d="M8 11v18l8-9" fill="#4285f4"/><path d="M32 11v18l-8-9" fill="#34a853"/>',
      );
    case "meet":
      return wrapSvg(
        '<rect x="8" y="12" width="16" height="16" rx="3" fill="#00ac47"/><polygon points="26,16 32,12 32,28 26,24" fill="#00832d"/>',
      );
    case "chat":
      return wrapSvg(
        '<path d="M8 12c0-2.2 1.8-4 4-4h16c2.2 0 4 1.8 4 4v12c0 2.2-1.8 4-4 4H14l-6 5V12z" fill="#00ac47"/><circle cx="15" cy="18" r="2" fill="#fff"/><circle cx="20" cy="18" r="2" fill="#fff"/><circle cx="25" cy="18" r="2" fill="#fff"/>',
      );
    case "drive":
      return wrapSvg(
        '<polygon points="15,8 25,8 33,22 23,22" fill="#ffba00"/><polygon points="7,22 15,8 20,16 12,30" fill="#0066da"/><polygon points="12,30 28,30 33,22 17,22" fill="#00ac47"/>',
      );
    case "calendar":
      return wrapSvg(
        '<rect x="8" y="9" width="24" height="22" rx="3" fill="#fff" stroke="#1a73e8" stroke-width="2"/><rect x="8" y="9" width="24" height="7" fill="#1a73e8"/><text x="20" y="26" font-size="11" font-weight="bold" fill="#1a73e8" text-anchor="middle">31</text>',
      );
    case "contacts":
      return wrapSvg(
        '<rect x="9" y="8" width="22" height="24" rx="2" fill="#1a73e8"/><circle cx="20" cy="16" r="4" fill="#fff"/><path d="M14 26c0-3.3 2.7-6 6-6s6 2.7 6 6" fill="#fff"/>',
      );
    case "translate":
      return wrapSvg(
        '<polygon points="8,10 22,10 18,24 8,24" fill="#4285f4"/><polygon points="18,16 32,16 28,30 18,30" fill="#ea4335"/><text x="14" y="19" font-size="10" fill="#fff" font-weight="bold">G</text><text x="24" y="25" font-size="10" fill="#fff" font-weight="bold">文</text>',
      );
    case "photos":
      return wrapSvg(
        '<path d="M20 20v-9a4.5 4.5 0 0 1 9 0v9z" fill="#ea4335"/><path d="M20 20h9a4.5 4.5 0 0 1 0 9h-9z" fill="#fbbc05"/><path d="M20 20v9a4.5 4.5 0 0 1-9 0v-9z" fill="#34a853"/><path d="M20 20h-9a4.5 4.5 0 0 1 0-9h9z" fill="#4285f4"/>',
      );
    case "shopping":
      return wrapSvg(
        '<rect x="9" y="12" width="22" height="20" rx="3" fill="#ea4335"/><path d="M14 12V9a6 6 0 0 1 12 0v3" fill="none" stroke="#ea4335" stroke-width="3"/><circle cx="20" cy="22" r="3" fill="#fff"/>',
      );
    case "docs":
      return wrapSvg(
        '<path d="M12 8h11l7 7v17a2 2 0 0 1-2 2H12a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z" fill="#4285f4"/><polygon points="23,8 30,15 23,15" fill="#a1c2fa"/><line x1="15" y1="20" x2="25" y2="20" stroke="#fff" stroke-width="2"/><line x1="15" y1="24" x2="25" y2="24" stroke="#fff" stroke-width="2"/><line x1="15" y1="28" x2="21" y2="28" stroke="#fff" stroke-width="2"/>',
      );
    case "sheets":
      return wrapSvg(
        '<path d="M12 8h11l7 7v17a2 2 0 0 1-2 2H12a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z" fill="#0f9d58"/><polygon points="23,8 30,15 23,15" fill="#87ceac"/><line x1="15" y1="20" x2="25" y2="20" stroke="#fff" stroke-width="1.5"/><line x1="15" y1="24" x2="25" y2="24" stroke="#fff" stroke-width="1.5"/><line x1="20" y1="18" x2="20" y2="26" stroke="#fff" stroke-width="1.5"/>',
      );
    case "slides":
      return wrapSvg(
        '<path d="M12 8h11l7 7v17a2 2 0 0 1-2 2H12a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z" fill="#f4b400"/><polygon points="23,8 30,15 23,15" fill="#fbe7a1"/><rect x="15" y="19" width="10" height="7" rx="1" fill="#fff"/><rect x="17" y="21" width="6" height="3" fill="#f4b400"/>',
      );
    case "keep":
      return wrapSvg(
        '<circle cx="20" cy="20" r="14" fill="#fbbc04"/><path d="M16 23h8v2h-8z" fill="#fff"/><path d="M17 26h6v1h-6z" fill="#fff"/><path d="M20 13a5 5 0 0 0-4 8c.8.8 1 1.5 1 2h6c0-.5.2-1.2 1-2a5 5 0 0 0-4-8z" fill="#fff"/>',
      );
    case "storytold":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#7928ca"/><path d="M14 26l6-12 6 12-4-2-2 4-2-4z" fill="#ffffff"/><circle cx="20" cy="14" r="2.5" fill="#ffd700"/>',
      );
    case "store":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#0f9d58"/><path d="M10 14h20v4l-2 8H12l-2-8v-4z" fill="#ffffff"/><path d="M16 14v-2a4 4 0 0 1 8 0v2" fill="none" stroke="#ffffff" stroke-width="2"/>',
      );
    case "bench":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#e53e3e"/><circle cx="20" cy="22" r="9" fill="none" stroke="#ffffff" stroke-width="2.5"/><line x1="20" y1="22" x2="25" y2="17" stroke="#ffffff" stroke-width="2" stroke-linecap="round"/>',
      );
    case "shell":
      return wrapSvg(
        '<rect x="6" y="6" width="28" height="28" rx="8" fill="#0078d4"/><rect x="11" y="11" width="8" height="8" rx="1.5" fill="#ffffff"/><rect x="21" y="11" width="8" height="8" rx="1.5" fill="#ffffff"/><rect x="11" y="21" width="8" height="8" rx="1.5" fill="#ffffff"/><rect x="21" y="21" width="8" height="8" rx="1.5" fill="#ffffff"/>',
      );
    default:
      return wrapSvg(
        '<circle cx="20" cy="20" r="16" fill="#4285f4"/><text x="20" y="25" font-size="14" fill="#fff" font-weight="bold" text-anchor="middle">G</text>',
      );
  }
}
