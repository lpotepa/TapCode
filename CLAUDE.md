# TapCode

Mobile-first programming language learning app. Users build code by tapping token chips instead of typing. Rust is the launch language.

## Tech stack

- **Framework:** Dioxus 0.6 (Rust UI framework — compiles to iOS, Android, web, desktop from one codebase)
- **Language:** Rust
- **UI syntax:** RSX (JSX-like macro for Rust)
- **State management:** Dioxus signals (`Signal<T>`, `Memo<T>`) — fine-grained reactivity like SolidJS
- **Styling:** TailwindCSS via Dioxus built-in Tailwind support
- **Backend:** Supabase (Postgres + Auth + PostgREST)
- **HTTP client:** reqwest with `json` + `wasm` features
- **Serialization:** serde + serde_json
- **Web output:** WASM static site on Cloudflare Pages
- **Animations:** CSS transitions for simple interactions; `use_coroutine` + `Signal<Vec<Particle>>` at 60fps for particle effects

## Platform targets

| Platform | Feature flag | Build command | v1 scope |
|----------|-------------|---------------|----------|
| iOS | `mobile` | `dx serve --platform ios` | Yes |
| Android | `mobile` | `dx serve --platform android` | Yes |
| Web | `web` | `dx serve --platform web` | Yes |
| Desktop | `desktop` | `dx serve --platform desktop` | No |

Production builds: `dx bundle`. Mobile binaries < 5 MB. Web WASM + HTML + CSS < 300 KB gzipped.

## Cargo features

```toml
[features]
default = []
web     = ["dioxus/web"]
desktop = ["dioxus/desktop"]
mobile  = ["dioxus/mobile"]
server  = ["dioxus/server", "dep:tokio"]
```

## Architecture

### Core principle: language-agnostic engine

The app is a generic **token assembly engine**. It has zero language-specific logic. All language knowledge lives in **language packs** (data, not code). Adding a new language = adding a new language pack JSON bundle. No recompile needed.

### Language pack contents

Each language pack provides:
- Metadata (ID, display name, version, primary color hue, icon, tagline)
- Token category definitions (keywords, types, symbols, identifiers, etc. with chip colors)
- Contextual state machine (which chip groups highlight after which token sequences)
- Syntax highlighter config (token-to-color mappings for code canvas)
- Module tree (ordered modules, each with ordered challenge IDs)
- Challenge bundle (all challenge JSON objects)
- Compiler adapter config (for validation pipeline)

### Token categories (universal across languages)

| Category | Role |
|----------|------|
| Keywords | Language-reserved words (`fn`, `let`, `mut`) |
| Macros/decorators | Meta constructs (`println!`, `vec!`) |
| Types | Type names and constructors (`i32`, `String`, `Vec<T>`) |
| String literals | Quoted values (`"Hello"`) |
| Symbols & punctuation | Structural characters (`{`, `}`, `;`, `->`) |
| Identifiers/vars | User-defined names (`x`, `count`, `name`) |
| Numbers | Numeric literals (`42`, `3.14`) |

### State management pattern

- **Global state** (XP, streak, unlocked modules): `use_context` from root provider
- **Screen state** (assembled tokens, current lesson, feedback): component-local `use_signal`
- **Optimistic updates**: mutate local signal immediately, write to Supabase async via background `use_coroutine`
- **Offline queue**: `Signal<Vec<PendingMutation>>` — flushes when connectivity restored. Serialized to file (mobile) or `localStorage` (web) to survive restarts

### Routing

```rust
#[derive(Routable, Clone)]
enum Route {
    #[route("/")]              Home {},
    #[route("/lesson/:id")]    Lesson { id: String },
    #[route("/module/:id")]    ModuleMap { id: String },
    #[route("/profile")]       Profile {},
    #[route("/compose/:module_id")] FreeCompose { module_id: String },
}
```

## Supabase

**Project ref:** `dynffktqahqfelfriwfm`

### Authentication

- Anonymous session on first launch (no signup form)
- JWT stored in platform secure storage (Keychain/Keystore/localStorage)
- Optional upgrade to named account (email/OAuth) merges anonymous progress

### Database schema

```sql
-- one row per user
create table user_state (
  id             uuid primary key references auth.users,
  created_at     timestamptz default now(),
  total_xp       int default 0,
  current_streak int default 0,
  longest_streak int default 0,
  last_active    date
);

-- per-language progress
create table language_progress (
  id               uuid primary key default gen_random_uuid(),
  user_id          uuid references user_state,
  language_id      text not null,
  xp               int default 0,
  active_module    int default 1,
  unlocked_modules int[] default '{1}',
  unique (user_id, language_id)
);

-- every attempt at a challenge
create table challenge_attempts (
  id           uuid primary key default gen_random_uuid(),
  user_id      uuid references user_state,
  challenge_id text not null,
  language_id  text not null,
  correct      boolean not null,
  attempt_num  int not null,
  attempted_at timestamptz default now()
);

-- one row per calendar day the user was active
create table streak_log (
  user_id uuid references user_state,
  day     date not null,
  primary key (user_id, day)
);
```

RLS enabled on all tables: `user_id = auth.uid()` — users read/write own rows only.

### Supabase client pattern

```rust
struct SupabaseClient {
    base_url: String,
    anon_key: String,
    jwt: Signal<Option<String>>,
    http: reqwest::Client,
}
```

Communicates via PostgREST REST API + Auth REST endpoints using reqwest.

## Challenge schema

```json
{
  "id": "rust-m2-c3",
  "language": "rust",
  "module": 2,
  "position": 3,
  "title": "Reassign a mutable variable",
  "prompt": "Reassign the variable x to the value 10",
  "hint_concept": "...",
  "hint_structural": "_ = _;",
  "fragment_type": "statement",
  "answer": ["x", "=", "10", ";"],
  "answer_variants": [],
  "expected": { "exit_code": 0, "stdout": "", "stderr": "" },
  "chips": [
    { "group": "identifiers", "tokens": ["x", "y", "count"] },
    { "group": "values",      "tokens": ["10", "0", "42"] },
    { "group": "symbols",     "tokens": ["=", ";", ":"] },
    { "group": "keywords",    "tokens": ["let", "mut"] }
  ],
  "xp": 20,
  "validation": { "status": "verified", ... }
}
```

Challenge IDs namespaced by language: `rust-m2-c3`, `go-m1-c1`.

Fragment types: `expression`, `statement`, `fn_def`, `type_def`, `program`.

## Challenge validation pipeline

Compiler adapters (one per language, Docker image) verify every challenge answer compiles and produces expected output before publish. Language-agnostic orchestrator selects adapter by `challenge.language`.

Rust adapter wraps fragments:
- `expression`: `fn main() { let _ = {FRAGMENT}; }`
- `statement`: `fn main() { {FRAGMENT} }`
- `fn_def`: `{FRAGMENT}\nfn main() {}`
- `program`: `{FRAGMENT}`

Sandbox: no network, read-only FS except /tmp, 5s timeout, 512MB memory, single-use containers.

CI blocks PRs if any challenge fails validation.

## Rust curriculum (v1: modules 1-10)

1. **First output** — `println!`, `print!`, `dbg!`, format args (5 challenges)
2. **Variables and bindings** — `let`, `mut`, type annotations, shadowing (5 challenges)
3. **Functions** — params, return types, implicit return, calling (5 challenges)
4. **Control flow** — `if`/`else`, `loop`, `for`, `while`, `match` (5 challenges)
5. **Ownership basics** — `String::new`, `String::from`, move, clone, borrow (5 challenges)
6. **Structs** — define, instantiate, field access, `impl`, constructors (5 challenges)
7. **Enums and match** — variants, data variants, `Option<T>`, `Result<T,E>` (5 challenges)
8. **Error handling** — `unwrap`, `expect`, `?` operator, match on Result (5 challenges)
9. **Traits intro** — derive Debug, impl Display, generics, custom traits (5 challenges)
10. **Collections** — `Vec`, `push`, iteration, `HashMap`, `get` (5 challenges)

Modules 1-3 are free. Module 4+ requires purchase.

## Monetization

- No ads ever
- Lifetime: $9.99, Annual: $4.99/yr, Monthly: $1.99/mo
- iOS/Android: In-App Purchase. Web: Stripe checkout (no monthly option on web in v1)

## Core UX mechanics

### Token assembly

User taps chips to build code sequences. Tapping a chip appends to canvas. Tapping a placed token backtracks to that point. Dedicated undo button removes last token. "Check" compares against canonical answer(s).

### Contextual chip behavior

After each tap, a per-language state machine determines which chip groups highlight/dim. Rules live in the language pack. No hard-coded language logic in UI.

### Answer validation (v1)

Token sequence matching against canonical `answer` array. Multiple `answer_variants` supported for semantic equivalence. v2 adds WASM on-device compilation.

### Wrong answer feedback

Diff-style: green = correct position, red = wrong token, ghost = missing. Copy never says "Wrong" — uses "Not quite", "Almost", "Close".

### Hint system

- Tier 1 (concept): plain-language explanation, no code (-5 XP)
- Tier 2 (structural): ghost overlay on canvas showing shape with blanks (-5 XP)
- Third tap: skip option (no XP cost, added to revisit queue)
- First hint per session per lesson is free

### Progression

- 20 XP per correct first-attempt, 10 XP per retry-correct
- Daily streak: 1 challenge/day. One auto-freeze/week
- Module badges on completion (always awarded, no performance gate)
- Free compose mode unlocked per completed module

## Animation timings

| Moment | Duration |
|--------|----------|
| Chip tap (scale) | 80ms |
| Token appears in canvas | 120ms |
| Undo token | 100ms |
| Correct answer (green pulse) | 480ms |
| XP bounce | 300ms |
| Confetti burst | 600ms |
| Wrong answer diff flash | 300ms |
| Feedback panel slide-up | 250ms |
| Module badge drop | 1500ms |
| Streak fill | 400ms |
| Chip group dim/highlight | 150ms |

Sound is off by default. Haptics use iOS patterns (`.light`, `.error`, success pulse, double pulse).

## Accessibility

- `aria_label` on all chip elements for VoiceOver/TalkBack
- System dynamic type (relative units, not hardcoded px)
- Color is never the sole state indicator — always paired with icon or text
- Full VoiceOver/TalkBack traversal test required before App Store submission

## Web-specific

- Keyboard: 1-9 selects chips, Enter submits, Backspace undoes, Tab moves groups
- Responsive: vertical stack (mobile) → two-column at 768px+
- Service worker for offline after first load
- Deep links: `tapcode.dev/rust/m2/c3` routes to specific challenges
- Anonymous JWT in localStorage — loss = progress loss unless account upgraded
- Hosted on Cloudflare Pages (static). Marketing site at root, app at `/app`

## Mobile constraints (Dioxus)

- Android: NDK + cargo-ndk cross-compilation setup required
- Hot-patching Rust code is experimental; RSX/CSS hot-reload is stable
- Haptics/system fonts/safe area insets require JNI (Android) or ObjC (iOS) calls — wrap behind platform abstraction trait
- Test on real devices early (start at end of module 2 implementation)

## Key design principles

1. **No language-specific code in the core app** — everything is driven by language pack data
2. **Optimistic UI** — local state updates instantly, network writes are async
3. **Never punishing** — wrong answers are "Not quite", streaks are about showing up not perfection
4. **Immediate competence** — first correct answer within 90 seconds of app open
5. **Variable reward compression** — always a reward close in the future (tap → challenge → module)

## Version roadmap

- **v1**: Modules 1-6, token assembly, streaks, XP, badges, iOS + Android + web simultaneous launch
- **v1.5**: Modules 7-10, named accounts (email/OAuth), Supabase Realtime leaderboard, share URLs, free compose, second language pack
- **v2**: WASM on-device compilation, user-generated challenges, community packs, spaced repetition review
