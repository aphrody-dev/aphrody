export interface GoogleApp {
  id: string;
  name: string;
  url: string;
  iconType: string;
  badge?: string;
}

export const GOOGLE_APPS: GoogleApp[] = [
  {
    id: 'account',
    name: 'Compte',
    url: 'https://myaccount.google.com/?hl=fr',
    iconType: 'account'
  },
  {
    id: 'search',
    name: 'Recherche',
    url: 'https://www.google.com/?hl=fr',
    iconType: 'search'
  },
  {
    id: 'maps',
    name: 'Maps',
    url: 'https://maps.google.com/?hl=fr',
    iconType: 'maps'
  },
  {
    id: 'youtube',
    name: 'YouTube',
    url: 'https://www.youtube.com/?hl=fr',
    iconType: 'youtube'
  },
  {
    id: 'news',
    name: 'Actualités',
    url: 'https://news.google.com/?hl=fr',
    iconType: 'news'
  },
  {
    id: 'gmail',
    name: 'Gmail',
    url: 'https://mail.google.com/',
    iconType: 'gmail'
  },
  {
    id: 'meet',
    name: 'Meet',
    url: 'https://meet.google.com/',
    iconType: 'meet'
  },
  {
    id: 'chat',
    name: 'Chat',
    url: 'https://chat.google.com/',
    iconType: 'chat'
  },
  {
    id: 'drive',
    name: 'Drive',
    url: 'https://drive.google.com/',
    iconType: 'drive'
  },
  {
    id: 'calendar',
    name: 'Agenda',
    url: 'https://calendar.google.com/',
    iconType: 'calendar'
  },
  {
    id: 'contacts',
    name: 'Contacts',
    url: 'https://contacts.google.com/',
    iconType: 'contacts'
  },
  {
    id: 'translate',
    name: 'Traduction',
    url: 'https://translate.google.com/?hl=fr',
    iconType: 'translate'
  },
  {
    id: 'photos',
    name: 'Photos',
    url: 'https://photos.google.com/',
    iconType: 'photos'
  },
  {
    id: 'shopping',
    name: 'Shopping',
    url: 'https://www.google.com/shopping?hl=fr',
    iconType: 'shopping'
  },
  {
    id: 'docs',
    name: 'Docs',
    url: 'https://docs.google.com/document/',
    iconType: 'docs'
  },
  {
    id: 'sheets',
    name: 'Sheets',
    url: 'https://docs.google.com/spreadsheets/',
    iconType: 'sheets'
  },
  {
    id: 'slides',
    name: 'Slides',
    url: 'https://docs.google.com/presentation/',
    iconType: 'slides'
  },
  {
    id: 'keep',
    name: 'Keep',
    url: 'https://keep.google.com/',
    iconType: 'keep'
  }
];
