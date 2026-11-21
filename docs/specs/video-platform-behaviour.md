# Video platform UI: behaviour and layout specification

## Header

- Purpose: single source of truth for an engineer who has never seen the reference site, to build an original video-platform UI in GPUI for Aphrody (home feed, watch view, search results, creator page, short-form vertical viewer, side navigation).
- Method: observer role, date 2026-11-21. Reference was a large public video site, logged out, cookies declined, UI language was French (irrelevant to geometry). Tools: headless capture engine on the build host (text/links only), built-in browser pane with viewport emulation (1440x900, 1024x768, 768x900, 400x800, one 1920x1000 run) and in-page measurement (bounding rects, computed styles), real key presses with state read-back.
- Clean-room note: no wording, logos, icons, brand names, images, brand colours, titles or user content are reproduced. Features are named generically. All numbers are CSS pixels measured at 1x layout unless stated. Colours are given as roles only. The implementer must pick original icons, palette, typography and copy.
- Confidence tags: [M] measured this session; [O] observed visually; [U] could not be verified (see section 13).

## 0. Global shell [M]

- Top bar: fixed, height 56, spans full width, transparent over page background. Three zones: start (menu toggle 40x40 at x16, wordmark slot ~129 wide), center (search, 40 high, max width 732, centered in the viewport, not in the leftover space), end (about 225 wide: overflow menu, sign-in pill, optional create/notification buttons).
- Search box: pill, height 40; text field plus a trailing square-ish submit button; a clear (x) button appears when text is present and on narrow widths the field collapses to an icon (below about 500 px) that expands on tap.
- Page container: starts at y56. Left offset = 240 (expanded nav), 72 (mini nav), 0 (hidden/overlay). Content area is then offset by an additional 24 horizontal padding on list pages.
- Page background and surfaces follow the OS colour scheme (dark when OS is dark). Roles: base background, raised surface (cards, dialogs: 1 step lighter in dark), 10% white overlay for unselected chips and ghost buttons, primary text (near white in dark / near black in light), secondary text (mid grey), accent for live/recording/progress (a saturated red role), link/accent-blue role for text links.
- Typography: one humanist sans family with system fallback; root size 10 px (all sizes are rem-like). Scale used: 12/18 (meta, badges), 14/20 (chips, tabs, buttons weight 500), 16/22 and 18/26 (card/result titles), 20/28 weight 700 (watch title), 24/32 (creator name). Primary text weight 400-500, titles heavier in grid cards (700 in feed cards).
- Radii: thumbnails 12; chips 8; duration badge 4; avatars full circle; mini player 12; short-form player 12; dialogs 12.
- Top-level navigation is SPA style: URL changes without reload, scroll position restored on back.

## 1. Left navigation [M]

Three states.

- Expanded (240 wide, from y56 to bottom, scrolls independently): sections separated by 1 px divider, 12 px side inset. Entry: 204x40 (inset 12), icon 24 at x24, label 14/20, radius 10 on hover/selected, selected = 10% white fill, weight 500. Sections in order: primary (home, short-form, subscriptions, personal library), a sign-in prompt block (logged out) with outlined pill button, an explore section (music, films and TV, live, a "more" disclosure that expands), a "more from the platform" section, then settings/report/help/about links and a footer with legal links in 12/18 grey.
- Mini (72 wide, fixed): 4 entries only (home, short-form, subscriptions, personal library), each 64x76 centered with 4 px inset, icon 24 on top, label 10 px beneath, vertical pitch 76. No scroll.
- Hidden: 0 wide; the menu toggle opens it as an overlay drawer (240 wide) over a 60% black scrim; clicking the scrim, pressing Escape or selecting an entry closes it; focus is trapped while open.
- Which state where: home, search, creator page and short-form viewer show expanded or mini (persisted preference toggled by the menu button); watch view defaults to hidden/overlay at all widths. Below 1024 px expanded is not allowed inline: mini shows at 1024 (72 wide) and at 768 and below mini is removed entirely (overlay drawer only, menu button remains).
- Toggling expanded <-> mini shifts the page offset 240 <-> 72 with no re-flow animation of cards (grid re-computes columns).

## 2. Card anatomy (shared by feed, creator grid, related list, results) [M]

Vertical card (feed, creator page):
- Width is fluid per column; measured 420-421 wide with 16 gap on wide windows. Thumbnail 16:9 (420x236.8), radius 12, no border. Total card height 346.8 (feed, with 36 avatar) / 314 (creator grid, no avatar, 2-line title).
- Overlay on thumbnail: duration badge bottom-right, 8 px inset from corner, text 12/18 weight 500, padding 1x4, radius 4, background 60% black, white text. Variants: live badge (accent fill, text "LIVE" role), playlist/mix count badge, short badge. Watched-progress bar: 4 px high strip along the thumbnail bottom, accent fill over a translucent track.
- Below thumbnail, gap 12: row with channel avatar 36x36 circle (feed only), then text column (title, channel line, meta line), then a 40x40 overflow button (visible on hover or always on touch; hover state = 10% white circle). Text column width = card width - 36 - 12 - 40 gaps (measured 376.9 incl. menu).
- Title: 16-18 px, line-height 22-26, clamp 2 lines (3 lines were never observed), ellipsis, primary text colour.
- Channel line: 12/18 secondary text, 1 line, may carry a small verified glyph.
- Meta line: 12/18 secondary: "<views> - <relative time>", separator is a middle dot style delimiter with 4 px side margins; live items show "<n> watching".
- Hover (pointer, after ~500 ms dwell): thumbnail starts a muted inline preview with a thin accent progress line along the bottom edge; duration badge swaps for mute/captions mini buttons; leaving stops it. Card background does not change in the feed; in lists a 10% white rounded hover plate (radius 12) may appear.
- Focus: card link shows a visible 2 px focus ring on the thumbnail; Enter opens; menu button reachable via Tab as separate stop.

Horizontal row (search results, creator lists, related list) [M]:
- Results row: thumbnail 500x281.25 (16:9) at 1440, radius 12, then 16 gap, text column fills remaining width. Row pitch = thumbnail height + 16 (297). Title 18/26 primary, clamp 2 lines. Meta line 12/18 secondary under the title (views, age). Channel line: avatar 24 + name 12/18 + verified glyph. Snippet: 12/18 secondary, clamp 2 lines. Badges row (e.g. "new", "subtitles", quality tags) as small 12 px text chips with 4 px radius. Overflow button 40x40 top-right of the row.
- Related list in watch view: thumbnail 242.3x136.3 (16:9), card 387.8 wide, title 14/20 2-line clamp, channel/meta 12/18, row pitch about 136+8.
- Result variants seen in one list: ordinary video row, sponsored row (same layout plus a "sponsored" label and a destination host line, no avatar), playlist/mix row (thumbnail with stacked-layers top edge, count badge, 3 text lines of first entries), short-form shelf (horizontal grid of portrait cards), channel row (circle avatar 136, name, handle, subscriber count, subscribe pill).

## 3. Home feed [M for shell, O for feed]

- Observation limit: with cookies declined and no account the feed renders an empty-state card instead of videos. See section 9. Grid rules below come from the same grid component measured on the creator page at 768/1024/1440/1920 plus one logged-in-profile reading of the home grid (4 declared column slots, 3 realised, card 420.9, gap 16, content 1310 at a 1595 px viewport).
- Filter chip bar: sticky under top bar, height 56 (chip 32 high, 8 gap, top/bottom 12), starts at content left. Chip: radius 8, 14/20 weight 500, padding 0x12, unselected 10% white fill with primary text, selected inverted fill (light fill / dark text in dark theme and the reverse in light). First chip is an "all" chip. Row scrolls horizontally with no scrollbar; when overflowing, a 48 wide fade plus arrow button appears at the end(s); arrow scrolls by about 75% of visible width. Selecting a chip reloads the grid in place (grid shows skeletons, scroll resets to top, bar stays pinned).
- Grid: card target width about 420, gap 16 horizontal, 40 vertical between rows (row pitch 383 for 346.8 card = 36 more than card height). Columns = floor((content width + 16) / 436), minimum 1; content width = viewport - nav offset - 2 x 24 padding (about 16 in a 1595 reading).
- Infinite scroll: batch of 20-30 cards; a 60 px high spinner row sits at the bottom; next batch is requested when the spinner is about 1 screen height from the viewport bottom; the page height jump is about +7000 px for results lists per batch; no page-number control. Observed 8 successive loads without end.
- Loading skeleton: cards replaced by grey rounded rectangles (thumbnail 16:9 radius 12, avatar circle 36, two text bars of ~100% and ~60% width), pulsing is not required; shown for first paint and during chip switches.
- Refresh: no pull-to-refresh on desktop; clicking the wordmark returns to top and reloads feed.

## 4. Watch view [M at 1440, partly O]

Layout at 1440x900 with nav hidden: two columns inside a page 1424.8 wide: left column (primary) x16, width 1005 (989 content + 16 gutter); right column (secondary) x1021, width 403.8 (387.8 content + 16 right padding). Column top at y68 (12 under the top bar).
- Player (default mode): width 989 follows column; height follows the video aspect (16:9 gave 989x556.3; a 4:3 clip gave 989x741.8), with a max height of viewport height minus 169 in theater mode (731 at 900) and a min height of about 480 [O]. Radius 12, black letterbox background.
- Below the player: title 20/28 weight 700, up to 2 lines; owner row (avatar 40, name, subscriber count 12/18, subscribe pill 36 high) on the left; action row on the right (like/dislike pill group, share, save/download, overflow; each pill 36 high, 18 radius, 10% white fill, 8 gap). Row height 42.4. Description box: 12/18 or 14/20, collapsed to 2-3 lines with a "more" toggle, 10% white fill, radius 12, padding 12. Comments section: heading with count and a sort control, composer line (avatar 40 + underline field), threaded items with avatar 40, reply toggle, like button; loaded lazily on scroll into view; logged-out composer prompts sign-in.
- Secondary column: chip bar (all / same-creator / related, same chip spec) then a vertical list of horizontal cards (242x136 thumbnails). Infinite scroll appends in place.
- Theater mode (toggle): player becomes full-bleed 1424.8 wide, height 731 (viewport height - 169, capped by aspect), black side bars if the video is narrower. Below it, the title/description column returns to 989 wide on the left and the secondary column moves under the player at x1021 (top y811). Persisted preference.
- Fullscreen: element fills the display; controls overlay; Escape or toggle exits; page scroll locked. Could not be triggered by synthetic input [U].
- Miniplayer: opens on key or button and also when navigating away while a video plays. 400x225 video plus a 76 high strip (title, channel, play/pause, close, expand) = 400x301, pinned bottom-right 16 px from the right edge and about 16 from the bottom, radius 12, stays above page content, persists across home/search/creator pages, draggable not tested. Expand returns to the watch view; close stops playback.
- Autoplay-next: when a video ends the next recommended item starts after a short countdown overlay with cancel; a toggle in the control bar enables/disables it. Observed in session (next video began without input).
- Skeleton (first paint): black player rectangle with a faint centered play glyph, under it two grey title bars (about 70% and 40% width), a row of 5 small circles at the right (actions), an avatar circle plus two short bars, and a pill placeholder; right column empty until hydrated. Hydration took up to 25 s in the emulated browser.
- Narrow behaviour [U for exact thresholds]: at 1024 the player takes the full content width (990 measured at x24) and the secondary list is expected below the comments; two-column returns only above roughly 1000-1100 px.
- Error state: when a video is unavailable, a centered flat illustration (about 160 px), one-line headline "no longer available" role, and an outlined pill button "go to home". Top bar remains; nav hidden.

## 5. Player controls [M]

- Bottom control bar overlay: chrome-bottom 59 high (x12 inset, width = player - 24); scrubber container 6 high at the top of the bar (hit area 12+ px), buttons row 56 high beneath.
- Left group: play/pause 40x40, mute/volume 40x40 (volume slider expands horizontally on hover to about 52 px, value 0-100), time display "current / total" 12-13 px (115 wide), chapter title as a text button when chapters exist.
- Right group (each 48x40): autoplay switch, subtitles toggle, settings (menu: quality, speed, captions, audio), theater/default size toggle, miniplayer (when allowed), fullscreen. Casting button is conditional.
- Visibility: controls and gradient fade out after about 3 s of pointer idle during playback (class flips to an autohide state), return on mouse move, on pause, and on keyboard use; when paused they stay. Cursor hides with them.
- Scrubber: thin (3-4 px) track that thickens on hover (to about 6), three layers (loaded range lighter, played range accent, hover preview range), circular handle appears on hover, hover shows a time tooltip plus a thumbnail storyboard frame above the cursor (about 160x90) and the chapter name; click or drag seeks; dragging past the ends clamps.
- Click on video toggles play; double-click toggles fullscreen; double-tap left/right third seeks 10 s on touch.
- Overlays: centered spinner while buffering (about 64 px ring), a transient round icon in the center for play/pause/volume/seek feedback (fades about 0.5 s), captions rendered bottom-center in a translucent black box, 2 lines max, moves up when controls are visible.
- Sponsored interstitial (platform-specific, optional): pre-roll with a skip control bottom-right revealed after a countdown, a sponsor card bottom-left, and seeking disabled during it. Implementer may omit.
- Settings menu: popup anchored above the gear, width about 250, rows 40 high, panels slide horizontally between root and sub-menu, back arrow header.

## 6. Search results [M]

- Page container: left offset by nav, content column starts x264 at 1440 with expanded nav (240 + 24 padding), width 1136.8 (max about 1280 centered), rows stacked, no grid.
- Top: sticky filter chip row (all, short-form, unwatched, videos, recently uploaded, live) 32 high chips under the top bar, followed on the right by a "filters" button that opens a popover with columns (upload date, type, duration, features, sort by). Trailing "about these results" link.
- Spell-correction line ("did you mean" with the corrected query as a link) appears above the first row; queries with no strong match still return fuzzy results, so a hard "no results" state was not observed.
- Row stack uses the horizontal card spec (section 2). Mixed result types appear in order (sponsored, video, mix, short-form shelf, channel).
- Infinite scroll: 23 items initially, then batches of about 5-15 per trigger; 60 px spinner row at the bottom.
- Skeleton: top bar placeholders (search pill plus three circles) until hydration, then rows appear at once.
- Thumbnail width responds: 500 (>= about 1280 content), 444 at 1024, 352 at 768, 176 at 400. Text column takes the rest. Row layout never stacks vertically, even at 400.

## 7. Creator page [M]

- Banner: full content width minus margins; height 207 at 1440 (1284 content, ratio about 6.2:1), 138 at 1024, 103.5 at 768, 62 at 400; radius 16 [O]; page max content width 1284 centered.
- Header block (height 196 at 1440): avatar circle 160 at the left (120 at 768, 72 at 400), right of it: name 36/44 style bold with verified glyph, a meta line "handle - subscribers - video count" 14/20 secondary, a clamped 1-line description with "more" link, link line, and a subscribe pill 36 high. At 400 the subscribe pill becomes full width on its own row beneath.
- Tabs: row 48 high, 14/20 weight 500, 24 gap, selected tab has 2-3 px underline in primary text colour and full-strength text, others secondary; at overflow the row scrolls with an arrow button; search icon at the end of the row. Tabs observed: home, videos, short-form, live, podcasts, playlists, posts.
- Videos tab: sort chips (latest, popular, oldest) as in chip spec; grid of vertical cards without avatars (card 420 wide, 314.3 high, 16 gap). Columns: 3 at 1440 and 1920 (1308 grid), 2 at 1024 (872 grid) and 768 (card 311, grid 654), 1 at 400 (anomalous measured card 202 inside a 218 grid with 91 left offset; implementer should use a fluid single column).
- Mini nav stays visible at 1440 when the user collapsed it.
- Sticky behaviour: tabs do not stick; only the top bar is fixed.

## 8. Short-form vertical viewer [M]

- Layout at 1440x900 (mini nav): reel container 1272x804 at (72,64); player is 9:16: 452.3x804, centered in the container (x482), radius 12. Items stacked vertically with 16 gap (scroll step 820 px).
- Controls over the player, top, 48x48 each at y80 (16 from top): left cluster (play/pause x498, mute x554), right cluster (captions x774, more x822, fullscreen x870). Shown on hover or tap; hidden otherwise.
- Action rail: vertical column of 48x48 round buttons at x946 (12 right of the player), pitch 78, bottom-aligned; order: like (with count under it), comments (count), share, remix, then a small rounded-square channel/sound avatar 40 at the bottom. Count text 12/18 centered under each button.
- Metadata block: bottom-left, 378 wide at x88, y750 (outside the player at wide widths); contains an "auto-dubbed" style small label, channel avatar 32 + handle + subscribe pill (40 high, 95 wide), title 16 px clamped to 1 line with ellipsis at wide widths. Below 600 px it moves inside the player's lower-left (x42, width 260) over a bottom gradient.
- Progress: 3-4 px bar at the bottom edge of the player, accent fill, scrubbable on hover/drag.
- Navigation: down/up circular buttons 56x56 at the right edge (x1360, vertically centered, 8 px apart), hidden when only a single direction is available and hidden below 600 px. Arrow Down/Up keys, wheel and swipe move exactly one item; URL updates to the item id on each move (verified: Down changed id, Up restored it); items preload one ahead.
- Autoplay: item plays muted-capable, loops until user advances; tapping the video toggles pause; active item only plays.
- Narrow (400): reel 348x704 at x26 (26 side margin), video box 348x619 centered with black bars, action rail overlays the right edge at x314 (still 48x48), metadata overlays bottom, nav arrows removed, mini nav removed.
- Skeleton: black 9:16 rounded rectangle with a small centered play glyph and the same top bar placeholders.
- Comments open as a side panel (wide) or bottom sheet (narrow) [O].

## 9. Empty, loading and error states

| State | Behaviour |
|---|---|
| Home feed with no account and no history | One centered card (about 400 wide, radius 12, raised surface) with a headline, two lines of explanation and a full-width secondary button; no cards below |
| First paint of any page | Masthead placeholders (search pill, three circles) then page skeleton, then content; skeleton lasts 1-25 s in constrained environments |
| Chip switch | Grid replaced by skeletons, bar remains |
| Pagination | 60 px spinner row at list end |
| Unavailable video | Centered illustration, one-line headline, outlined "go home" pill |
| Search with weak match | Spell-suggest line plus fuzzy results; empty state not reachable by gibberish |
| Cookie/consent | Full-screen modal over a dimmed page with a language selector, long explanatory text, three actions (reject all, accept all, more options); reject keeps the site usable but disables history-based feed |

## 10. Responsive table (viewport width)

| Width | Nav | Top bar search | Feed/creator grid cols | Search row thumb | Watch layout | Creator header |
|---|---|---|---|---|---|---|
| >= 1536 (1920 sampled) | expanded 240 or mini 72, user choice | 732 max, centered | 3 (creator, content cap 1284); home uses the same rule up to a higher count [U] | 500 | two columns 989 + 404, theater available | avatar 160, banner 207 |
| 1440 | expanded 240 or mini 72 | 732 | 3 (creator); 3-4 (home) | 500 | two columns | avatar 160, banner 207 |
| 1024 | mini 72, expanded as overlay | 583 | 2 | 444 | single column expected, player 990 | avatar 160, banner 138 |
| 768 | no inline nav, overlay only | 331, chip bar shows scroll arrow | 2 (card 311) | 352 | single column [U] | avatar 120, banner 103 |
| 400 | overlay only | icon only (expands on tap) | 1 | 176 | single column [U] | avatar 72, banner 62, subscribe full width |

Page side padding: 24 at all list widths (8 at 400 in the creator header). Mobile-specific chrome (bottom tab bar) was not shown at 400; the site stays in its desktop template.

## 11. Interaction and keyboard table

Verified this session means state change was read back after a real key press. Others are listed as expected behaviour of the reference [U].

| Key / gesture | Context | Result |
|---|---|---|
| k | player focus or page | toggle play/pause (verified) |
| space | player | toggle play/pause [U]; space on a focused button activates it |
| m | player | toggle mute (verified) |
| Arrow Down / Up | watch view | volume -/+ about 4-5% (verified down: 0.87 to 0.83) |
| Arrow Left / Right | watch view | seek -/+ 5 s [U: blocked by pre-roll] |
| j / l | watch view | seek -/+ 10 s [U: blocked by pre-roll] |
| 0-9 | watch view | jump to 0-90% of duration [U] |
| Home / End | watch view | start / end [U] |
| < and > (shift+, shift+.) | watch view | playback speed step [U] |
| , and . | paused | frame step [U] |
| t | watch view | toggle theater (verified: 989x556 to 1425x731, secondary column moves below) |
| i | watch view | toggle miniplayer (verified: 400x301 bottom-right, navigates to home) |
| f | watch view | fullscreen (not triggerable by synthetic input) [U] |
| c | watch view | captions toggle [U] |
| Esc | overlays, fullscreen, drawer | closes the top-most |
| Down / Up | short-form viewer | next / previous item, URL id updates (verified) |
| Tab / Shift+Tab | everywhere | logical order: skip link, menu, search, sign-in, nav entries, chips, cards; each card exposes thumbnail link, title link, channel link, menu button |
| / | any page | focus search [U] |
| Enter in search | search box | navigates to results |
| Pointer hover on card | feed | muted inline preview after dwell |
| Pointer idle 3 s | player | controls fade |
| Wheel / swipe | short-form viewer | one item per gesture |
| Click on scrubber | player | seek to point |
| Drag scrubber handle | player | live seek, release commits |

Focus: a 2 px ring in the primary text colour with 2 px offset on keyboard focus only; none on pointer click.

## 12. Acceptance checklist

1. Shell: top bar 56 high and fixed; search centered at max 732, 40 high; menu toggle 40x40 at x16.
2. Nav expanded 240 / mini 72 / hidden; entries 204x40 expanded and 64x76 mini; menu toggle switches expanded <-> mini on list pages and opens overlay drawer on watch view.
3. Overlay drawer closes on Escape, scrim click and entry select, and traps focus.
4. Vertical card: 16:9 thumbnail with radius 12, duration badge bottom-right (12/18, 60% black, radius 4), avatar 36, title clamps to 2 lines, meta 12/18, overflow button 40x40.
5. Grid column count follows floor((content + 16) / 436) with 16 gap; verify at 768, 1024, 1440, 1920.
6. Chip bar 32 high chips, 8 gap, radius 8, selected inverted; overflow arrows appear and scroll; selection reloads grid with skeletons.
7. Infinite scroll appends in batches and shows a 60 px spinner row; never loses scroll position; handles 8+ batches without stutter (virtualise rows in GPUI).
8. Search rows: 16:9 thumbnail up to 500 wide, 16 gap, title 2-line clamp, snippet 2-line clamp, sponsored/playlist/shelf variants render.
9. Watch view at 1440: primary 989 content + secondary 388 content, player 989 wide and 16:9 height 556; theater 1425x(viewport-169) and secondary moves under the player.
10. Control bar 59 high, scrubber 6 high container, buttons 40/48; controls hide after about 3 s idle and re-show on move, pause and key press.
11. Scrubber hover shows time tooltip, loaded vs played ranges, thickening, handle; drag seeks.
12. Keys k, m, Up/Down volume, t, i behave as in section 11; unverified keys are implemented per the same table.
13. Miniplayer 400x225 + 76 strip bottom-right, persists across pages, expand and close work.
14. Autoplay-next with countdown and a disable toggle.
15. Skeleton for first paint on watch, list and short-form views; unavailable-video state with illustration and home button.
16. Creator page: banner (207/138/103/62), avatar (160/120/72), tabs 48 high with underline, videos grid 3/2/2/1 columns, sort chips.
17. Short-form viewer: 9:16 player 452x804 centered with 12 radius; action rail 48x48 buttons pitch 78 at 12 right of player; Down/Up and wheel move one item and update the URL; narrow layout overlays rail and metadata with 26 side margins.
18. Light and dark both follow the OS preference; selected chip inverts in both; no hard-coded colours, only role tokens.
19. All interactive items reachable by Tab with visible focus ring; Enter/Space activate; Esc closes layers.
20. Reduced motion: preview autoplay, skeleton shimmer and slide transitions disabled when the platform setting requests it [U, reference not tested].

## 13. Not observed and why

- Logged-in home feed grid at several widths: feed is empty when logged out with cookies declined; one reading from a logged-in browser profile gave card 420.9, gap 16, 3 realised columns at 1595 px, but window resizing was not possible there; max column count at very wide widths unknown.
- Watch view at 1024, 768, 400: page hydration stalled under emulation; only the skeleton and a full-width player (990 at 1024) were seen.
- Seek keys, number keys, speed and frame-step keys, c, "/" and fullscreen: a pre-roll interstitial blocked seeking and fullscreen needs a real user gesture; listed from reference behaviour, not measured.
- Hover preview timing, scrubber storyboard, settings menu panel sizes, comment panel in short-form, drag of miniplayer: not exercised (hover-only or requires interaction beyond read-only scope).
- Real light-mode geometry equals dark; only roles and selected-chip inversion were confirmed.
- Typography of the capture host showed zoomed text in one browser; type sizes above come from the clean emulated browser (root 10 px).
