import { describe, expect, it } from 'bun:test';
import { getSuggestions, POPULAR_SUGGESTIONS } from '../src/data/suggestions';
import { GOOGLE_APPS } from '../src/data/apps';

describe('Aphrody (Google Replica)', () => {
  it('should load popular suggestions', () => {
    expect(POPULAR_SUGGESTIONS.length).toBeGreaterThan(5);
    expect(POPULAR_SUGGESTIONS).toContain('aphrody');
  });

  it('should return recent history when query is empty', () => {
    const suggestions = getSuggestions('');
    expect(suggestions.length).toBeGreaterThan(0);
    expect(suggestions[0].isHistory).toBe(true);
  });

  it('should return filtered suggestions matching query', () => {
    const suggestions = getSuggestions('aphrody');
    expect(suggestions.length).toBeGreaterThan(0);
    expect(suggestions[0].text.toLowerCase()).toContain('aphrody');
  });

  it('should have all official Google Apps defined with URLs and icons', () => {
    expect(GOOGLE_APPS.length).toBeGreaterThanOrEqual(15);
    const names = GOOGLE_APPS.map((a) => a.name);
    expect(names).toContain('Recherche');
    expect(names).toContain('Maps');
    expect(names).toContain('YouTube');
    expect(names).toContain('Gmail');
    expect(names).toContain('Drive');
    expect(names).toContain('Agenda');
  });
});
