# Alex redesign — goose2 UI (2026-04-23)

**Author:** Tulsi + Claude (brainstorming session, 2026-04-23)
**Designer of record:** Alex
**Source of truth:** `Goose Brand — Q2 2026` Figma file
(`e43a6gyBVn1SdARFkZpN0N`, frame `113:1274`)
**Target scope:** `ui/goose2` (Tauri + React + Tailwind), NOT `ui/desktop`
**Target branch:** `tulsi/visual-design`, cut fresh from `origin/main` — does
NOT evolve `tulsi/visual-exploration` (the abandoned frosted-glass branch)

---

## 1. Summary

Alex's Q2 2026 reference set redefines goose2's visual posture as
**restrained, editorial, translucent-but-flat**. The abandoned
`tulsi/visual-exploration` branch over-committed on frosted-glass texture
(shadows, inset highlights, saturated backdrop filters). Alex's reset
brings the same translucent palette back but drops all the depth cues
and shadow stacks. Hierarchy comes from size + opacity + letter-spacing
on a **single font weight** (Cash Sans Regular 400), not bold.

Against Ghost UI's fingerprint, this is a realignment on two dimensions
the abandoned branch diverged on (`surface-hierarchy`, `elevation`) and
one intentional divergence on a third (`font-sourcing` — Ghost ships no
fonts, we adopt Cash Sans as a consumer override, which is the legal use
of that dimension). Net: closer to Ghost, not further.

## 2. Scope

### 2.1 In scope

**Foundation layer** — tokens, Cash Sans loading (Regular 400 + Regular
Italic 400), canvas treatment, dot grid tuning, shadow removal,
typography system, light-scheme lock during exploration.

**Shell** — `AppShell`, `TopBar` (breadcrumb pattern), `Sidebar` (approach
A: preserve current IA, re-skin in place), new `GlobalComposerPill`
component mounted at app level.

**Four feature views** (re-skin, no feature additions beyond the global
composer):
1. **Chat** — bounded rounded card on canvas, composer attached to card
   bottom, card flexes width around the existing Context panel.
2. **Skills** — static decorative category hero tiles + individual skill
   card rework with tag pill (3-color cycle by index).
3. **Agents / Personas** — cutout-figure card treatment, grid layout.
4. **Sessions / Recents** — `SessionCard` and `SessionHistoryView` re-skin.

### 2.2 Out of scope (explicit)

Each item below gets its own spec later if wanted.

- Loader redesign (goose glyph screen)
- Projects page with world cubes (ties into `anaghavi/cube-explo`)
- Search page with hero display typography
- Right-side Context panel styling (Alex flagged WIP — we only design
  the chat card's layout to flex around it)
- Dark mode — `color-scheme: light` locks the app to light during this
  exploration
- Real widget canvas (drag/drop/persistence) on Home — static
  placeholder images only
- Real category feature on Skills — static decorative tiles only
- Real tall-figure avatar system on Agents — decorative cutouts cycled
  by persona index only
- Real "Pinned" feature in sidebar (approach A skips it entirely)
- Tauri window chrome radii (10px top / 50px bottom observed in Figma,
  unclear if intentional) — flag during implementation

### 2.3 Success criteria

1. `pnpm tauri dev` shows each in-scope page matching foundation-token
   level with Alex's Figma references.
2. Typecheck stays at the 4 pre-existing `origin/main` errors in
   `src/features/extensions/api/extensions.ts` and
   `src/shared/api/acpApi.ts` — SDK drift, not ours.
3. Branch is cut fresh from `origin/main`; `tulsi/visual-exploration`
   is not an ancestor.
4. This spec committed alongside the foundation change (commit 1).
5. Commit history reads as five logical steps, not one grab-bag.

**Durable constraint:** *functionality must remain intact*. Every
re-skin is surface-level; stores, hooks, APIs, and data models are
untouched.

## 3. Foundation tokens

Three-layer cascade mirroring Ghost UI:

```
raw hex/number → semantic token → Tailwind @theme inline utility
```

### 3.1 Cash Sans loading

Self-hosted from Block CDN, only Regular 400 and Regular Italic 400.

```css
@font-face {
  font-family: 'Cash Sans';
  src: url('https://cash-f.squarecdn.com/static/fonts/cashsans/v2/CashSans-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://cash-f.squarecdn.com/static/fonts/cashsans/v2/CashSans-RegularItalic.woff2') format('woff2');
  font-weight: 400;
  font-style: italic;
  font-display: swap;
}
```

Exact URLs per weight to be pulled from the Regulator PR 2308 pattern
during implementation (`gh api 'repos/squareup/regulator-fe/contents/client/src/assets/fonts.css?ref=refs/pull/2308/head' --jq '.content' | base64 -d`).

Fallback stack: `'Cash Sans', system-ui, -apple-system,
BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`.

`font-display: swap` — system-ui stack renders until Cash Sans loads,
then swaps. No invisible-text flash.

### 3.2 Color tokens

```css
:root {
  /* canvas + surfaces */
  --canvas:            #dedede;
  --surface-chrome:    rgba(255, 255, 255, 0.5);  /* sidebar, settings pill */
  --surface-composer:  rgba(255, 255, 255, 0.2);  /* global chat pill */
  --surface-button:    #f5f5f5;                   /* icon/text buttons inside pills */
  --surface-card:      #ffffff;                   /* chat card */
  --surface-tile:      #f5f5f5;                   /* skill cards */
  --surface-install:   #dedede;                   /* install-on-hover button */

  /* text */
  --text-default:      #242424;
  --text-muted:        #7f7f7f;
  --text-title:        #19191a;

  /* tag pills (Skills; cycle by index) */
  --pill-pink:         #eec2ea;
  --pill-olive:        #cdcda1;
  --pill-blue:         #bcc6f4;
  --pill-neutral:      #ffffff;                   /* new-skill empty state */

  /* dot grid */
  --dot-color:         rgba(37, 37, 37, 0.15);
  --dot-size:          1px;
  --dot-spacing:       24px;
}
```

### 3.3 Typography scale

```css
:root {
  --font-sans: 'Cash Sans', system-ui, -apple-system, BlinkMacSystemFont,
               'Segoe UI', Roboto, sans-serif;

  --text-label:  10px;   /* section labels (opacity 0.25), timestamps      */
  --text-body:   14px;   /* nav items, row titles, buttons, placeholder    */
  --text-input:  16px;   /* chat pill placeholder, skill card description  */
  --text-title:  24px;   /* "Tulsi's World" and equivalents                */
}
```

Usage rules:
- `10px` + `opacity: 0.25` = section label ("Pinned", "Recents", grouped
  date headers like "Today")
- `10px` + no opacity = timestamps at `color: var(--text-muted)`
- `14px` = nav, buttons, tag pill text, session titles
- `16px` + `line-height: 20px` = multi-line body text (skill / agent
  descriptions, chat composer placeholder)
- `24px` + `line-height: 0.96` + `letter-spacing: -0.04em` = world title
  and breadcrumb
- **Never `font-weight: bold` or `500`.** One weight only. Hierarchy is
  size + opacity + letter-spacing.

### 3.4 Radii

```css
:root {
  --radius-chrome:    16px;   /* sidebar */
  --radius-composer:  40px;   /* chat input pill (global + attached)   */
  --radius-pill:      9999px; /* all buttons, tag pills                */
  --radius-tile:      20px;   /* skill cards, category tiles           */
  --radius-card:      TBD;    /* chat bounded card — extract from frame 227:859 */
}
```

### 3.5 Spacing

Base grid is 4px. Ghost's existing spacing tokens stay — we use:

```
--spacing-1: 4px
--spacing-2: 8px    (icon-in-pill gap, nav item row gap)
--spacing-3: 12px   (pill horizontal padding)
--spacing-4: 16px   (section gap)
```

### 3.6 Dot grid utility

```css
.bg-dot-grid {
  background-color: var(--canvas);
  background-image: radial-gradient(circle,
                                    var(--dot-color) var(--dot-size),
                                    transparent var(--dot-size));
  background-size: var(--dot-spacing) var(--dot-spacing);
}
```

Replaces the abandoned branch's 16px / `rgba(26,26,26,0.18)` version.

### 3.7 No-shadow rule

Explicit: **no `box-shadow` on any new or re-skinned surface.** Depth
comes from luminance contrast with canvas (50% white reads brighter
than 87% grey) and size affordance.

Ghost's `--shadow-*` tokens stay defined — components we do not touch
(`popover.tsx`, `dropdown-menu.tsx`, `dialog.tsx`, `alert-dialog.tsx`,
`select.tsx`) continue consuming them. We just:
1. Do not add new shadow usage.
2. Remove shadow from surfaces we actively re-skin.

### 3.8 Light-scheme lock (temporary)

```css
html { color-scheme: light; }
```

Forces light palette regardless of OS dark mode while this branch is in
flight. Remove when dark mode is designed (out of this spec).

### 3.9 Dropped from the abandoned branch

- `.glass-panel`, `.glass-card` utilities
- `--panel-glass-*`, `--card-glass-*` tokens
- `saturate()`, `backdrop-filter: blur(...)` from all surfaces
- Inset top-highlight + ambient-drop shadow stacks
- The 7-color `PILL_PALETTE` helper in `SkillsView.tsx` — replaced with
  3-color cycle (pink / olive / blue)

### 3.10 `@theme inline` utility exposure

```css
@theme inline {
  --color-canvas:            var(--canvas);
  --color-surface-chrome:    var(--surface-chrome);
  --color-surface-composer:  var(--surface-composer);
  --color-surface-button:    var(--surface-button);
  --color-surface-card:      var(--surface-card);
  --color-surface-tile:      var(--surface-tile);
  --color-text-default:      var(--text-default);
  --color-text-muted:        var(--text-muted);
  --color-text-title:        var(--text-title);

  --radius-chrome:    var(--radius-chrome);
  --radius-composer:  var(--radius-composer);
  --radius-tile:      var(--radius-tile);
}
```

## 4. Shell

### 4.1 AppShell

**File:** `src/app/AppShell.tsx`

Current structure (TopBar → flex-row(Sidebar | Resizer | Main) →
StatusBar) stays. Only the root surface treatment changes.

Changes:
- Root: `bg-background text-foreground` → `bg-dot-grid text-default`
- Add `html { color-scheme: light }` (via globals.css)
- Mount the new `GlobalComposerPill` component inside the flex-row,
  absolutely positioned bottom-right, *conditionally hidden* when
  `activeView === "chat"` (where the composer is attached to the chat
  card instead)
- All logic (resize, collapse, session management, settings, keyboard
  shortcuts) untouched

### 4.2 TopBar

**File:** `src/app/ui/TopBar.tsx`

Biggest delta in the shell. Rewrite (small file, 27 lines → ~45 lines).

```tsx
<header className="h-12 pl-20 pr-3 flex items-center gap-3" data-tauri-drag-region>
  <h1 className="font-sans text-[24px] leading-[0.96] tracking-[-0.04em] text-[--text-title]">
    {/* TODO: wire to active Project.title when Projects page ships */}
    Tulsi's World
    {currentPageLabel && (
      <>
        <span className="text-[--text-muted] opacity-60"> / </span>
        <span className="text-[--text-muted]">{currentPageLabel}</span>
      </>
    )}
  </h1>
  <div className="flex-1" />
  <Button variant="pill-muted" onClick={onSettingsClick}>Settings</Button>
</header>
```

Specifically:
- Drop `bg-background/80 backdrop-blur-sm` → transparent (canvas shows)
- Height: `h-10` → `h-12` (48px — accommodates 24px title)
- **Do not** migrate sidebar-toggle or search icons into topbar
  (approach A keeps them in their current locations)
- User/avatar icon button → "Settings" text pill
  (`14px opacity 0.7`, `bg-[--surface-button]`, `rounded-full`, `h-8`,
  `px-3`)
- Preserve `data-tauri-drag-region` and `pl-20` traffic-light clearance

`currentPageLabel` derives from `activeView`:
- `home` / `chat` → no label (title alone)
- `skills` → "Skills"
- `agents` → "Agents"
- `session-history` → "Session History"
- `projects` → "Projects" (when that view ships later)

### 4.3 Sidebar — approach A (preserve IA, re-skin in place)

**File:** `src/features/sidebar/ui/Sidebar.tsx`

Current IA stays: GooseIcon + collapse-btn → inline search input → Home
→ nav items (Agents, Skills, Session History) → `SidebarProjectsSection`
(per-project grouped sessions). No "New Chat" nav item added; no
"Pinned" section; no search migration to topbar; no flattening of
Projects → Recents.

Token swaps:
- Root container: `bg-background border border-border rounded-xl` →
  `bg-[--surface-chrome] rounded-chrome` (no border, no shadow)
- Text colors → new tokens
- Cash Sans applied via root `font-family` cascade (no per-component
  font choice)
- Inline search input restyled to borderless transparent bg, 14px Cash
  Sans

Section label audit: at implementation time, inspect
`SidebarProjectsSection` — any section label ("Projects", grouped
headers, etc.) becomes `10px Cash Sans Regular, opacity 0.25,
color: var(--text-default)` with a 1px horizontal rule above at
`border-color: var(--color-gray-200)`.

Per-session row: if it renders a status indicator, swap to a 6px green
dot when `isSessionRunning()` returns true; 10px timestamp below title
at `color: var(--text-muted)`.

### 4.4 GlobalComposerPill (new component)

**File:** `src/shared/ui/GlobalComposerPill.tsx` (new, ~80 lines)

Mounted at `AppShell` level. Hidden when `activeView === "chat"`.
Appears as a floating pill at `bottom-6 right-6`:

```
Shape:    rounded-full (40px radius), 482x68px max, responsive
Surface:  bg-[--surface-composer]  /* rgba(255,255,255,0.2) */
Padding:  left 30px for placeholder; right cluster of mic + send pills
Font:     16px Cash Sans Regular, color: black, opacity: 0.7
Buttons:  mic (16px icon, bg-[--surface-button], rounded-full, h-8 w-8)
          send (arrow, bg-[--surface-button], rounded-full, h-8 w-10)
```

Placeholder: a single universal "Start a conversation" string,
regardless of `activeView`. Earlier brainstorming (2026-04-23 spec
draft) proposed per-page contextual placeholders; design review
2026-04-27 chose a single neutral placeholder so the pill reads as a
universal compose surface rather than implying page-specific behavior.
The component's prop signature dropped `activeView` accordingly.

Functionality: type + send → AppShell's `handleGlobalCompose(text)`:
1. Create a new chat session via `createNewTab(DEFAULT_CHAT_TITLE)`
   (which switches `activeView` to `"chat"` and sets the active session
   internally).
2. **Auto-submit** the typed text as the first message (no draft-seed
   intermediate step). Implementation: store a `pendingFirstMessage`
   on the chat store keyed by sessionId; `useChatSessionController`
   consumes it on session-init via useEffect, calls `sendMessage`,
   and clears the field.

Why auto-submit: design review 2026-04-27. The pill's value over
"navigate to chat, then type" is one-shot send. Intermediate "draft
filled, click send" reads as a redundant extra step.

Mic = voice dictation (wire to existing `useVoiceDictation` hook if
trivially accessible; if not, defer mic as no-op for this pass).

Input styling: kill native browser focus chrome with
`appearance-none border-0 outline-none focus:outline-none focus:ring-0`
on the `<input>`. macOS WebKit defaults add a focus ring + border that
break the pill's translucent restraint.

## 5. Chat page

### 5.1 Bounded card layout

**File:** `src/features/chat/ui/ChatView.tsx`

Current flex-sibling pattern between ChatView and `ChatContextPanel`
already handles "card flexes when panel opens" correctly — we just
wrap the left column in a bounded card.

```tsx
<div className="relative flex h-full min-w-0 p-4">
  <div className="flex flex-1 flex-col min-w-0
                  bg-[--surface-card] rounded-[var(--radius-card)]
                  overflow-hidden">
    <MessageTimeline ... />
    <LoadingGoose ... />
    <ChatInput ... />
  </div>
  <ChatContextPanel ... />
</div>
```

Changes:
- Add `p-4` canvas padding on the outer flex so the card floats
- Wrap conversation + composer in a card surface (`#ffffff`, rounded,
  `overflow-hidden`, no border, no shadow)
- Drop `pr-1` from the inner column (the old 4px gutter is replaced
  by canvas padding)
- `ChatContextPanel` unchanged — it continues to width-eat; the card
  reflows naturally

**Card radius:** TBD — extract from Chat frame `227:859` during
commit 2. Likely 20px.

**Message internals are NOT restyled in this spec.** `MessageTimeline`
and its children (user bubbles, assistant messages, tool-call chips,
Thinking indicator) inherit new fonts/colors via token cascade but
keep their current shape. Restyling message bubbles to Alex's
right-aligned-user-pill + plain-assistant-text pattern is a separate
spec.

### 5.2 ChatInput re-skin (surface only)

**File:** `src/features/chat/ui/ChatInput.tsx` (508 lines — ~10-20
lines of outer container styling change)

- Outer container bg: transparent (sits on card's own white)
- Remove any border / shadow from composer container
- Cash Sans via root cascade
- Functionality (drafts, attachments, mentions, voice dictation,
  persona picker, model picker, project picker) — all intact

### 5.3 ChatContextPanel

**Files:** `src/features/chat/ui/ChatContextPanel.tsx`, `ContextPanel.tsx`,
`ChatView.tsx`

Alex flagged the panel content styling as WIP — that's preserved as-is.
The structural change in this spec is that the panel **lives inside the
chat card**, opening from the right edge of the card itself rather than
as an external sibling beside it.

ChatView's outer flex changes from `[card | panel]` to `[card[
conversation | panel ]]`:
- The chat card becomes a flex-row at the top level
- Inside the card, a `flex-1 flex-col` column holds conversation +
  composer
- `<ChatContextPanel>` is now a sibling of that column (still inside the
  card)

The panel's own surface is dropped — no `rounded-xl`, no `border`, no
`bg-background`. It picks up a `border-l border-[var(--color-gray-200)]`
divider so it reads as a distinct section of the card without competing
with the card's white surface. Internal sub-cards (Workspace / Changes
/ Extensions) keep their existing styling.

The 200ms width transition still works — the mechanism is unchanged
(width 0 → 364px on the panel's outer wrapper); only its parent moved
from "outer ChatView flex" to "chat card flex-row." The conversation
column flex-shrinks the same way it did before.

The toggle button continues to use absolute positioning relative to
ChatView's outer `relative` div, which keeps it at the top-right corner
of the chat card across both panel states.

### 5.4 Home view — static widget placeholders

**File:** `src/features/home/ui/HomeView.tsx` (new, or add to
`AppShellContent.tsx`)

When `activeView === "home"`, render a static composition of Alex's
widgets:
- Blurry blue "world" cube, centered
- Analog clock top-right
- Single human cutout figure, reused at three positions with different
  sizes / placements (Alex's Landing frame uses one figure PNG
  positioned three ways — visual variety comes from placement, not
  multiple cutouts)
- Blank white sticky note with red dot, bottom-left

Asset pipeline:
1. During commit 1, fetch Alex's asset URLs from the Landing frame
   (`110:66`) export via Figma MCP (valid for 7 days from
   2026-04-23 — re-fetch if we go past 2026-04-30).
2. Save to `src/assets/home/` with descriptive names:
   - `world-cube.png` — blurry cube photo
   - `figure.png` — single cutout (reused at 3 positions)
   - `clock.svg` — Figma exported as SVG (extension matches content,
     not the URL suffix)
   - `sticky-note.svg` — Figma exported as SVG
3. Render via `<img>` tags absolutely positioned with percentage
   offsets (preserves proportional placement across viewport sizes;
   accepts some overlap on small windows).

No drag. No persistence. No widget abstraction. Pure visual dressing;
real widget canvas (drag / drop / persistence) is a separate spec.

## 6. Skills

### 6.1 Tag pills (individual skill cards)

Three colors, cycled by index mod 3:

| Index | Color           |
|-------|-----------------|
| 0     | `--pill-pink`   |
| 1     | `--pill-olive` |
| 2     | `--pill-blue`  |

Pill text is the **skill name itself** (e.g., `agent-browser`,
`g-drive`, `data-dog`). Not category.

Pill geometry:
- `h-5` (20px), `px-6 pb-3` padding (`padding: 0 6px 3px 6px`)
- `rounded-full`
- 14px Cash Sans Regular, `color: #19191a`
- `gap-10` (10px)

### 6.2 Skill card

**File:** `src/features/skills/ui/SkillsView.tsx` (427 lines — major
rework of the card render)

```
┌──────────────────────────┐  260×260
│ [tag-pill]               │  tag 20px from top-left
│                          │
│   Description text,      │  ml-21 mt-53 (20+33)
│   222px wide,            │  16px Regular #7f7f7f, line-height 20
│   up to ~7 lines before  │
│   overflow truncation    │
│                          │
│                [Install] │  hover/focus only
└──────────────────────────┘  bg #f5f5f5, rounded-[20px]
```

- Card surface: `bg-[--surface-tile]` (#f5f5f5), `rounded-tile` (20px),
  no border, no shadow
- Install button (shown on `group-hover`): bottom-right inside card,
  `bg-[--surface-install]` (#dedede), `rounded-full`, 14px "Install" text
  at `opacity: 0.7`, `h-8`

### 6.3 New-skill empty state card

Same 260×260 footprint, same `#f5f5f5` tile, same `rounded-[20px]`.
Contents:
- Tag pill `new-skill` at top-left (20px offsets), `bg-white` (different
  from colored pills)
- Italic placeholder text at ml-21 mt-53:
  *"Describe your new skill, e.g. \"Summarize any webpage into 3 bullet
  points when given a URL.\""* (Cash Sans Regular Italic 400)
- Arrow-right submit button bottom-right: `bg-[--surface-install]`,
  `rounded-full`, 40×32, arrow icon

### 6.4 Category hero tiles — REMOVED at visual review (2026-04-27)

Originally specced as 7 category hero tiles + 1 new-skill empty-state
slot at the top of the Skills view. Implementation landed in Phase 3
working tree but was removed at the dev-server visual review on
2026-04-27: at production fidelity the cropped photo backgrounds read
as visually chaotic (notably the People tile's cropped face), and
since categories are not a real feature in goose2, they could not
justify their visual weight. The new-skill empty-state slot continues
to live as the always-first card in the skill grid (§6.3).

The asset-export fallback infrastructure that was provisioned for the
Research tile (CSS gradient prop on `CategoryHeroTile`) is no longer
needed. The fallback PATTERN itself (§10.1) remains valid for any
future asset-resolution decisions.

Deferred to a future spec when categorization is a real feature with
filtering / browsing semantics. Re-fetch Figma assets at that point
(originals expired 7 days after 2026-04-23 anyway, per the spec's
asset-export pipeline).

### 6.5 Toolbar — moved to TopBar (revised 2026-04-27)

At the Phase 3 visual review, Tulsi directed the action buttons to
move out of the Skills page body and into the global TopBar (right of
the breadcrumb, before Settings). The inline page-body header
("Skills" / "Reusable instructions for your AI agents") and the
SearchBar above the grid were both removed at the same time — the
breadcrumb in TopBar already names the active view, and the page
content can speak for itself without a sub-header.

Buttons (all `bg-[--surface-button]`, `rounded-full`, `h-8`):
- List view (icon-only, 36×32) — decorative; no list-mode toggle yet
- Sort (icon-only, 36×32) — decorative; no sort flow yet
- Import (text pill with upload icon) — triggers hidden file input
  in `SkillsView` via the topbar-actions context
- Add New (text pill with plus icon) — opens `CreateSkillDialog`

Wiring pattern: a small `TopBarActionsProvider` lives at AppShell
level, exposing `useSetTopBarActions()`. The active view (currently
only `SkillsView`) pushes its action JSX on mount and clears on
unmount; the JSX closes over the view's local state and refs so file
inputs and dialog handlers stay co-located with the view that owns
them. `TopBar` consumes via `useTopBarActions()` and renders the
result before its Settings pill.

The decorative List view + Sort buttons mirror the Install button
pattern: `tabIndex={-1}` so they're skipped in keyboard navigation
and they don't ship to production with no flow attached. When real
sort/list-mode features land, wire them in this same slot.

### 6.6 Bottom fade gradient

At the bottom of Skills (and other vertically-scrolling pages with the
Global Composer Pill), a 195px tall gradient:

```css
background: linear-gradient(to bottom,
                            rgba(222,222,222,0) 0%,
                            var(--canvas) 100%);
backdrop-filter: blur(1.5px);
```

This softens the last row of cards where they meet the floating
composer.

## 7. Agents / Personas

### 7.1 Persona card body — rebuilt

**File:** `src/features/agents/ui/PersonaCard.tsx` (161 → 130 lines —
full render replacement; functionality preserved)

```
[cutout figure]           ~110px × ~220px (portrait)
                           object-fit: contain, no surface
─────────────              1px horizontal rule, width 149px
[Name-pill]                bg [--surface-button], rounded-full, h-5,
                           14px Cash Sans Regular #19191a, px-6 pb-3
Description text           Renders persona.systemPrompt with
                           line-clamp-2; 16px Regular,
                           color: [--text-muted], width 149px
                           (the data model has `systemPrompt` not
                           `description`; original card already
                           rendered this field as the body paragraph)
```

No card surface. No border, no background, no padding on the cell.
Just the figure, rule, pill, and text — directly on canvas.

Active state (`isActive === true`): subtle background tint on the whole
cell container (not the photo) — `bg-black/[0.03]` on a cell with
enough padding to show the tint. Replaces the old `ring-1 ring-ring`.

Menu trigger (edit/duplicate/delete/export): `MoreVertical` icon
positioned `absolute top-2 right-2`. Visibility uses opacity, not
`display`, so the menu stays in the DOM tab order:
`opacity-0 transition-opacity focus-within:opacity-100
group-hover:opacity-100`. This preserves keyboard accessibility — Tab
reaches the kebab, focus-within reveals it. The kebab's
`<Button>` keeps `onClick={(e) => e.stopPropagation()}` and
`onKeyDown={(e) => e.stopPropagation()}` so triggering the menu
doesn't also fire the card's `onSelect`.

Aria label uses the existing `t("card.options")` key (no new i18n
strings introduced for the redesign).

### 7.1.1 Toolbar — moved to TopBar (revised 2026-04-27)

Mirroring the Skills page (§6.5), at the Phase 4 visual review the
inline page-body header ("Agents" + subtitle), SearchBar, and the
inline "Import" / "New Persona" buttons were removed. The same
TopBarActionsProvider context Skills uses now serves Agents:

- AgentsView calls `useFileImportZone()` directly (lifted up from
  PersonaGallery), holding `dropHandlers` + `isDragOver` locally
- AgentsView pushes a 2-button action set ("Import", "New Persona")
  to the TopBar slot via `useSetTopBarActions()` on mount; cleared on
  unmount
- PersonaGallery becomes a passive grid renderer that accepts
  `dropHandlers` + `isDragOver` props; the section wrapper carries the
  drop handlers so the entire gallery is a drop target with a
  `ring-2 ring-ring ring-offset-2` highlight when `isDragOver`
- The dashed-border "Create new" card that previously sat as the last
  grid cell is removed entirely; New Persona moves to TopBar
- Grid container bumped `max-w-5xl` → `max-w-7xl` so 4+ persona
  columns fit on wider windows (already matched `auto-fill, minmax`)

No new i18n strings introduced. Existing keys reused:
`common:actions.import`, `view.newPersona`.

### 7.2 Grid layout

**File:** `src/features/agents/ui/PersonaGallery.tsx`

Grid template: 5 columns by N rows (responsive: 2-5 columns based on
container width). Cell dimensions ~295px wide × 441px tall (figure +
rule + pill + desc + gap).

**Skip the "Less than 5" row-of-4 figures special case.** Always render
as a grid — one responsive layout for any persona count. Removes a
conditional render branch.

### 7.3 Cutout figure (single, shared)

Alex's Figma `Agents — More than 5` frame (`114:1485`) uses **one**
decorative cutout figure asset, rendered at 10 positions across two
rows with different crops — not 5 distinct figures as the earlier
brainstorm assumed. Inspection of the underlying image references
(on 2026-04-23) confirmed a single `imgImage6090` constant reused.

Pull that single asset via Figma MCP during commit 1. Save to
`src/assets/agents/figure.png` (one file). Every persona renders this
same image identically — no hashing, no index assignment. Visual
uniformity is intentional: it signals that real per-persona avatar
integration is a separate spec, rather than implying the current state
is the final design.

User-uploaded avatars (via `AvatarDropZone`, `useAvatarSrc`) — data
remains intact, but **the visual for this branch ignores uploaded
avatars** in favor of the shared decorative cutout. Real tall-figure
avatar integration = separate spec.

## 8. Sessions / Recents

Alex did not draw a dedicated Session History page frame. Sidebar
Recents is the only session-list treatment. `SessionHistoryView.tsx`
re-skin extrapolates from established patterns.

### 8.1 SessionCard

**File:** `src/features/sessions/ui/SessionCard.tsx`

Current card: `rounded border p-4` with icons, title, metadata, snippet,
dropdown menu. Rendered inside grouped date sections.

Re-skin target:
- Title: 14px Cash Sans Regular, `color: [--text-default]`
- Metadata (persona, project, updatedAt): 10px at `color: [--text-muted]`
- Snippet (when present from search): 14px, `color: [--text-muted]`
- Surface: subtle fill (`bg-[--surface-tile]` = #f5f5f5),
  `rounded-xl` (12px), `p-4`, no border, no shadow.
  Revised at visual review (2026-04-27): the original spec called for
  no surface, but at production fidelity the cards read as
  undifferentiated text rows. Subtle fill restores card-as-discrete-
  unit reading without competing with the dot-grid canvas.
- Hover: `bg-[--surface-install]` (#dedede) — same token Skills'
  Install button uses; one shade darker than the resting fill
- Menu trigger: `MoreHorizontal`, `opacity-0 group-hover:opacity-100
  focus-within:opacity-100` (visible on hover, keyboard-tabbable)

Functionality (select, rename, archive, unarchive, duplicate, export) —
unchanged.

### 8.1.1 Import — moved to TopBar (revised 2026-04-27)

Mirroring Skills + Agents (§6.5, §7.1.1), the page-body header
(h1 + subtitle) and the inline "Import session" button are removed.
"Import" moves to the global TopBar via `useSetTopBarActions()`. The
hidden file input (`fileInputRef`) stays co-located in
`SessionHistoryView` next to its `handleImportSession` change handler;
the TopBar button just triggers `fileInputRef.current?.click()`
through a closure pushed into the topbar-actions context.

The `<BottomFade />` shared component (extracted to
`src/shared/ui/BottomFade.tsx` from Skills + Agents) is rendered at
the end of the scrollable container — same soft-mask gradient pattern
across all three pages.

### 8.2 SessionHistoryView

**File:** `src/features/sessions/ui/SessionHistoryView.tsx`

- Background: canvas + dot grid (inherited from AppShell)
- Grouped date headers ("Today", "Yesterday", "Earlier this week"):
  title-case, 1px horizontal rule above, 10px opacity-25 label text —
  matches sidebar Pinned/Recents divider pattern
- Page-level `SearchBar`: borderless transparent bg, 14px Cash Sans
- Page padding: `p-8`
- Structure (search + grouped sections + list) unchanged
- Grouping logic (`groupSessionsByDate`) unchanged

### 8.3 Sidebar Recents (already documented in 4.3)

Per approach A: preserve sidebar IA; apply token-level re-skin to the
existing `SidebarProjectsSection`; section labels in title-case with
divider-above rule where present.

## 9. Implementation sequencing

Five sequential commits on branch `tulsi/visual-design`, cut fresh from
`origin/main`. Each commit has a dev-server visual checkpoint before
moving to the next.

### Commit 1 — Foundation

- This spec file (`ui/goose2/docs/superpowers/specs/2026-04-23-alex-redesign-design.md`)
- `globals.css` token additions (section 3)
- Cash Sans + Italic font-face loading
- `color-scheme: light` lock
- `AppShell` root changes (bg-dot-grid, GlobalComposerPill mount)
- `TopBar` rewrite (breadcrumb title, Settings text pill, transparent)
- `Sidebar` token-level re-skin (approach A)
- `GlobalComposerPill` new component
- Asset download script execution: fetch Home widgets, Skills categories,
  Agents cutouts from Figma MCP URLs to `src/assets/{home,skills,agents}/`

Biome pre-commit hook satisfied: commit bundles spec markdown with
substantial code changes.

### Commit 2 — Chat

- `ChatView` bounded card wrapping
- `ChatInput` outer container re-skin
- `HomeView` (or extension of `AppShellContent`) with static widgets
- Verify `ChatContextPanel` flex-resize still works
- Extract chat card radius from Figma frame `227:859` and commit
  concrete value to `--radius-card`

### Commit 3 — Skills

- Hero category tile row (8 static decorative tiles)
- Skill card rework (260×260, tag pill cycle, hover Install)
- New-skill empty state card
- Inline toolbar above grid
- Bottom fade gradient

### Commit 4 — Agents

- `PersonaCard` body replacement (figure + rule + pill + desc)
- `PersonaGallery` grid template adjustment
- Figure assignment by id-hash mod 5
- Hover dropdown trigger
- Replace `ring-1 ring-ring` active state with subtle bg tint

### Commit 5 — Sessions

- `SessionCard` surface-less re-skin
- `SessionHistoryView` page padding + grouped date headers
- Sidebar Recents inline polish (if any leftover tuning)

## 10. Risks, open questions, deferrals

### 10.1 In-flight risks

| Risk | Mitigation |
|------|-----------|
| Figma MCP asset URLs expire 7 days from 2026-04-23 | Re-fetch if implementation extends past 2026-04-30; spec documents the clock |
| Window corner radii (10px top / 50px bottom) — unclear if intentional | Flag during implementation; test Tauri window decorations; skip if artifact |
| Chat card radius — `TBD` in tokens | Extract from Chat frame `227:859` during commit 2 |
| Dot grid alpha (0.15 tentative from pixel-measure) | Dev-time tune via single CSS var |
| Cash Sans Italic exact URL | Grab from Regulator PR 2308 pattern during commit 1 |
| Typecheck baseline — 4 pre-existing SDK errors on `origin/main` | Not counted as regression |
| `backdrop-filter` on sidebar during drag-resize | No backdrop-filter used anywhere — risk negated |
| Figma MCP asset export returns wrong format (SVG/GIF bytes saved with `.png` extension) or animated / oversized content | **Asset-export fallback pattern:** when Figma's export for a given node is problematic — wrong format, animated when static expected, missing, or oversized — do NOT chase the exact Figma bytes. Use a clean in-code placeholder instead (CSS gradient, generic static image you generate or source). Rename any downloaded file to the extension that matches its actual content (verify with `file`). Document the fallback inline at the usage site so a future reader knows it's intentional. First applied 2026-04-23: Research category tile (original export was a 5MB animated GIF) rendered as a green/earth-toned CSS gradient; three home/skills SVGs were renamed to `.svg` to match their actual bytes. |

### 10.2 Caveats to flag when Alex reviews

- Home widgets are static visual placeholders — real drag / drop /
  persistence is a separate spec
- Skills category tiles are decorative — real category feature is a
  separate spec
- Research category tile uses an in-code CSS gradient placeholder
  (Figma's export was an animated GIF; we followed the asset-export
  fallback pattern — see §10.1)
- Persona figure is a single shared decorative cutout — real
  per-persona tall-figure avatar integration is a separate spec
- Dark mode is intentionally off during this branch (light-scheme lock)

### 10.3 Explicitly deferred (separate specs)

- Dark mode (Landing frame exists in dark; other pages not drawn)
- Loader redesign (goose glyph screen)
- Projects page + world cubes (ties into `anaghavi/cube-explo`)
- Search page + hero display typography
- Right-side Context panel styling (Alex WIP)
- Real "Pinned" feature in sidebar
- Real widget canvas (drag/drop/persistence) on Home
- Real category feature on Skills
- Real tall-figure avatar system on Agents
- Chat message-bubble restyle (right-aligned user pills, plain
  assistant text, etc.)
- Tauri window chrome radii (10/50)

## 11. Token reference card (quick lookup)

```
CANVAS            #dedede              background
SURFACE-CHROME    rgba(255,255,255,.5) sidebar, settings pill
SURFACE-COMPOSER  rgba(255,255,255,.2) global composer pill
SURFACE-BUTTON    #f5f5f5              icon/text buttons, name pills
SURFACE-CARD      #ffffff              chat bounded card
SURFACE-TILE      #f5f5f5              skill cards
SURFACE-INSTALL   #dedede              install-on-hover button

TEXT-DEFAULT      #242424              body
TEXT-MUTED        #7f7f7f              metadata, descriptions
TEXT-TITLE        #19191a              world title, breadcrumb primary

PILL-PINK         #eec2ea              skill tag (index %3 == 0)
PILL-OLIVE        #cdcda1              skill tag (index %3 == 1)
PILL-BLUE         #bcc6f4              skill tag (index %3 == 2)
PILL-NEUTRAL      #ffffff              new-skill empty state tag

DOT-COLOR         rgba(37,37,37,.15)   dot grid
DOT-SIZE          1px
DOT-SPACING       24px

RADIUS-CHROME     16px                 sidebar
RADIUS-COMPOSER   40px                 chat pill
RADIUS-PILL       9999px               buttons, tags
RADIUS-TILE       20px                 skill cards
RADIUS-CARD       TBD                  chat card (extract frame 227:859)

FONT-SANS         Cash Sans (400 + 400 italic) → system-ui fallback

TEXT-LABEL        10px                 section labels, timestamps
TEXT-BODY         14px                 nav, buttons, row titles
TEXT-INPUT        16px / lh 20         descriptions, placeholders
TEXT-TITLE        24px / lh 0.96 / -0.04em  world title
```

## 12. File inventory (what gets touched)

**Foundation (commit 1):**
- `src/shared/styles/globals.css` — token additions
- `src/app/AppShell.tsx` — root bg, GlobalComposerPill mount
- `src/app/ui/TopBar.tsx` — full rewrite
- `src/features/sidebar/ui/Sidebar.tsx` — token-level re-skin
- `src/shared/ui/GlobalComposerPill.tsx` — new
- `src/assets/home/` — new (4 files: `world-cube.png`, `figure.png`,
  `clock.svg`, `sticky-note.svg`)
- `src/assets/skills/` — new (7 files: 6 category `.png` photos +
  `tile-mask.svg`; Research has no file — rendered as CSS gradient)
- `src/assets/agents/` — new (1 file: `figure.png`, shared across
  all personas)

**Chat (commit 2):**
- `src/features/chat/ui/ChatView.tsx` — add card wrap + canvas padding
- `src/features/chat/ui/ChatInput.tsx` — outer surface re-skin
- `src/features/home/ui/HomeView.tsx` — new (or update
  `src/app/ui/AppShellContent.tsx`)

**Skills (commit 3):**
- `src/features/skills/ui/SkillsView.tsx` — category tiles row, card
  rework, empty state, toolbar, bottom fade
- `src/features/skills/ui/CategoryHeroTile.tsx` — new; supports either
  a `backgroundImageUrl` (masked photo) or a `gradient` CSS string
  (used for Research fallback)

**Agents (commit 4):**
- `src/features/agents/ui/PersonaCard.tsx` — body replacement (single
  shared figure, no hash/index assignment)
- `src/features/agents/ui/PersonaGallery.tsx` — grid template

**Sessions (commit 5):**
- `src/features/sessions/ui/SessionCard.tsx` — surface re-skin
- `src/features/sessions/ui/SessionHistoryView.tsx` — page polish +
  grouped headers

Totals: ~10 meaningfully-changed files + 2 new components
(`GlobalComposerPill`, `CategoryHeroTile`) + 12 downloaded asset files
+ 1 spec doc + 1 plan doc.

---

*End of spec.*
