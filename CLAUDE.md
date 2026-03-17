# TapCode

Mobile-first programming language learning app. Users build code by tapping token chips instead of typing. Rust is the launch language with 100 challenges across 20 modules covering beginner to advanced concepts.

## Project status

The app compiles and runs on web (`dx serve --platform web`). Core engine, validation pipeline, and full Rust curriculum are implemented with 79 unit tests + 2 integration tests (100 rustc compilations). UI renders via Dioxus 0.7 with a custom "Cosmic Code" dark theme.

**What's working:** onboarding flow, lesson screen with token assembly, contextual chip highlighting, answer validation with diff, feedback panels, hint system, XP/streak tracking, home/profile/module map/compose/paywall screens, confetti celebrations, module completion, full 100-challenge Rust curriculum verified by real `rustc`.

**What's not yet wired:** Supabase backend (traits defined, no live calls), real haptics (trait defined, no-op on web), push notifications, IAP/Stripe payments, service worker offline caching.

## Tech stack

- **Framework:** Dioxus 0.7 (Rust UI framework — compiles to iOS, Android, web, desktop)
- **Language:** Rust
- **UI syntax:** RSX (JSX-like macro for Rust)
- **State management:** Dioxus signals (`Signal<T>`) with `use_context_provider`/`use_context`
- **Styling:** Custom CSS design system (`assets/main.css`) — no Tailwind dependency
- **CSS loading:** `document::Stylesheet { href: asset!("/assets/main.css") }` in RSX (Dioxus 0.7 pattern)
- **Backend:** Supabase (Postgres + Auth + PostgREST) — not yet connected
- **HTTP client:** reqwest with `json` feature; `getrandom` with `js` feature for WASM
- **Serialization:** serde + serde_json
- **Web output:** WASM via `dx serve --platform web`
- **Animations:** CSS keyframe animations with custom properties for timing tokens

## Build commands

```bash
dx serve --platform web        # Dev server with hot-reload
cargo test                     # 79 unit tests (engine + validator)
cargo test --test challenge_compilation  # Compile all 100 challenges with rustc
cargo build --target wasm32-unknown-unknown --features web  # WASM build check
```

## Cargo features

```toml
[features]
default = []
web     = ["dioxus/web"]
desktop = ["dioxus/desktop"]
mobile  = ["dioxus/mobile"]
server  = ["dioxus/server", "dep:tokio"]
```

WASM requires `getrandom = { features = ["js"] }` and `uuid = { features = ["js"] }` in `[target.'cfg(target_arch = "wasm32")'.dependencies]`.

## Project structure

```
src/
  main.rs              # Entry point: launches App, provides global state, mounts Router
  lib.rs               # Library crate (for integration tests to import)
  models.rs            # All data types: LanguagePack, Challenge, ValidationResult, etc.
  engine.rs            # Language-agnostic engine: registry, validation, state machine, helpers
  validator.rs         # Challenge verification pipeline: CompilerAdapter trait + registry
  state.rs             # AppState: global state with XP, streak, progress, language pack
  route.rs             # Dioxus Router: Landing → Onboarding or Home, all screen routes
  components/          # Reusable UI components
    chip.rs            # TokenChip — tappable token with category color + animation
    canvas.rs          # CodeCanvas — assembled tokens with syntax highlighting + diff mode
    picker.rs          # TokenPicker — grouped chip grid with contextual highlighting
    action_bar.rs      # Check / Undo / Hint buttons
    feedback_panel.rs  # Slide-up panel for Correct / Wrong feedback
    xp_display.rs      # XP counter with bounce animation
    progress_bar.rs    # Segmented progress bar
    streak.rs          # 7-day dot strip with pulsing ring
    confetti.rs        # Particle burst celebration
    nav_shell.rs       # Bottom navigation bar
  screens/             # Full-page screens composed from components
    home.rs            # Streak display + Continue CTA + module list
    lesson.rs          # Core challenge loop: prompt → tap → check → feedback
    module_map.rs      # Challenge list per module + Free Compose entry
    profile.rs         # XP, streak stats, badges, account section
    compose.rs         # Sandbox mode: validate syntax, no correct answer
    onboarding.rs      # Splash → language select → skill check → first challenge
    paywall.rs         # Pricing screen (no dark patterns, back button works)
    module_complete.rs # Full-screen celebration with badge drop
  services/
    platform.rs        # Trait abstractions: HapticEngine, SecureStorage
assets/
  main.css             # Full design system: colors, animations, components, responsive
  data/
    rust_pack.json     # Complete Rust language pack (100 challenges, 20 modules)
tests/
  challenge_compilation.rs  # Integration test: compiles every challenge with real rustc
```

## Architecture

### Core principle: language-agnostic engine

The app has **zero language-specific logic**. Two parallel registries drive everything:

1. **`LanguagePackRegistry`** — loads language pack JSON data (challenges, chips, rules)
2. **`AdapterRegistry`** — loads compiler adapters for challenge verification

Adding a new language requires:
- One JSON file (language pack with challenges, chip categories, context rules)
- One struct implementing `CompilerAdapter` trait
- Two lines of registration code
- Zero changes to UI, engine, orchestrator, or tests

### Language pack registry (`engine.rs`)

```rust
// Build from embedded JSON — adding a language = adding one line
pub fn build_default_registry() -> LanguagePackRegistry {
    LanguagePackRegistry::from_embedded(&[
        ("rust", include_str!("../assets/data/rust_pack.json")),
        // ("go", include_str!("../assets/data/go_pack.json")),
    ])
}
```

Each `LanguagePack` contains: metadata, token categories (7 per language), contextual state machine rules, syntax highlight config, module tree, and challenges with scaffolds.

### Compiler adapter registry (`validator.rs`)

```rust
// Trait — each language implements this
pub trait CompilerAdapter: Send + Sync {
    fn language_id(&self) -> &str;
    fn file_extension(&self) -> &str;
    fn compiler_flags(&self) -> Vec<String>;
    fn wrap_fragment(&self, fragment: &str, ft: &FragmentType, scaffold: &str) -> Option<String>;
    fn validate_program_structure(&self, program: &str) -> Result<(), String>;
    fn run_command(&self) -> &str;
    fn timeout_seconds(&self) -> u32;
}

// Registry — adding a language = one line
pub fn default_registry() -> AdapterRegistry {
    let mut r = AdapterRegistry::new();
    r.register(Box::new(RustAdapter::new()));
    // r.register(Box::new(GoAdapter::new()));
    r
}
```

The `RustAdapter` splits scaffolds into outer (module-level: `use`, `struct`, `trait`) and inner (fn-level: `let` bindings) so fragments compile in correct scope.

### Contextual state machine

Pure function: `evaluate_context(tokens, rules, groups) → chip_group_states`. Rules live in the language pack JSON, not code. After each tap, determines which chip groups highlight (1.0 opacity) or dim (0.4 opacity).

### Challenge validation pipeline

Three layers, all language-agnostic:

| Layer | What it checks | When it runs |
|-------|---------------|-------------|
| **Static validation** | Tokens in chips, no prompt leaks, hints exist, IDs valid, adapter exists | `cargo test` (unit tests) |
| **Structural validation** | Wrapped program passes `validate_program_structure()` | `cargo test` (unit tests) |
| **Compiler verification** | Every answer compiles with the real compiler (rustc, go build, etc.) | `cargo test --test challenge_compilation` |

The integration test iterates ALL registered languages automatically. Adding Go = zero changes to the test.

### Challenge schema

```json
{
  "id": "rust-m2-c3",
  "language": "rust",
  "module": 2,
  "position": 3,
  "title": "Reassign a value",
  "prompt": "Change the value of x to 10 (assume x was declared as mutable)",
  "hint_concept": "Reassigning doesn't need the declaration keyword — just name, equals, value.",
  "hint_structural": "_ = _ ;",
  "fragment_type": "statement",
  "answer": ["x", "=", "10", ";"],
  "answer_variants": [],
  "expected": { "exit_code": 0, "stdout": "", "stderr": "" },
  "scaffold": "let mut x = 0;",
  "chips": [
    { "group": "identifiers", "tokens": ["x", "y", "count"] },
    { "group": "numbers", "tokens": ["10", "0", "42"] },
    { "group": "symbols", "tokens": ["=", ";", ":"] },
    { "group": "keywords", "tokens": ["let", "mut"] }
  ],
  "xp": 20,
  "explanation": "Reassignment uses = without let. Only works if declared with let mut."
}
```

Key fields:
- **`scaffold`**: Rust code prepended to make the fragment compile (imports, type defs, variable setup). The adapter places module-level items before `fn main` and variable bindings inside it.
- **`prompt`**: Must NEVER contain the answer code. Describe the goal, not the syntax.
- **`hint_structural`**: Uses `_` blanks, never reveals actual tokens.
- **`fragment_type`**: `expression`, `statement`, `fn_def`, `type_def`, `program` — determines how the adapter wraps it.
- **`chips`**: Must contain ALL answer tokens plus 1-3 distractors per group.

Challenge IDs namespaced by language: `rust-m2-c3`, `go-m1-c1`.

## Rust curriculum (100 challenges, 20 modules)

| # | Module | Concepts | Free? |
|---|--------|----------|-------|
| 1 | First Output | `println!`, `print!`, `dbg!`, format args | Yes |
| 2 | Variables & Bindings | `let`, `mut`, type annotations, shadowing | Yes |
| 3 | Functions | params, return types, implicit return, calling | Yes |
| 4 | Control Flow | `if`/`else`, `loop`/`break`, `for`, `while`, `match` | No |
| 5 | Ownership Basics | `String::new`, `String::from`, move, clone, `&` borrow, `&mut` | No |
| 6 | Structs | define, instantiate, field access, `impl`, `new()` constructor | No |
| 7 | Enums & Match | variants, data variants, `Option<T>`, `Result<T,E>`, `if let` | No |
| 8 | Error Handling | `unwrap`, `expect`, `?` operator, match Result, Result from main | No |
| 9 | Traits | derive Debug, impl Display, trait bounds, custom traits, impl Trait | No |
| 10 | Collections | `Vec`, `push`, for iteration, `HashMap`, `get` | No |
| 11 | Closures & Iterators | closure syntax, capturing, `map`, `filter`, `collect` | No |
| 12 | Lifetimes | `'a` annotations, struct lifetimes, multiple lifetimes, `'static` | No |
| 13 | Smart Pointers | `Box`, `Rc`, `Rc::clone`, `RefCell`, `borrow_mut` | No |
| 14 | Concurrency | `thread::spawn`, move closures, `Mutex`, `lock`, `Arc<Mutex<T>>` | No |
| 15 | Generics & Trait Objects | generic fn, trait bounds, `dyn Trait`, generic structs, `impl<T>` | No |
| 16 | Pattern Matching | destructure tuple/struct, `if let`, `while let`, match guards | No |
| 17 | Strings & Slices | `&str` vs `String`, `to_string`, `format!`, `contains`, `push_str` | No |
| 18 | Modules & Visibility | `mod`, `pub fn`, `use`, `super`, `pub use` re-exports | No |
| 19 | Macros | `assert_eq!`, `vec!`, `todo!`, `format!`, `assert!` | No |
| 20 | Advanced Ownership | `impl Drop`, `mem::replace`, `Cow`, type aliases, `impl Into` | No |

Modules 1-3 are free. Module 4+ requires purchase.

## Content authoring rules

When creating or editing challenges:

1. **Prompt must never leak the answer.** Describe the goal, not the syntax. BAD: "Write `println!("{}", x)`". GOOD: "Print the value of x using a format placeholder".
2. **Every answer token must exist in the challenge's chip groups.** The validator catches this.
3. **Include 1-3 distractor tokens per chip group** to make the challenge non-trivial.
4. **`hint_structural` uses `_` blanks** — never the actual tokens.
5. **`scaffold` provides compile context** — imports, type defs, variable declarations needed for the fragment to compile with `rustc`. Keep it minimal.
6. **Run `cargo test --test challenge_compilation`** after any challenge edit. It must pass.
7. **Challenge IDs follow `{language}-m{module}-c{position}`** format.
8. **Every challenge needs `explanation`** (shown on correct answer) and both hint tiers.

## Testing requirements

All code follows strict RED/GREEN TDD. Every Linear ticket has comprehensive test scenarios.

```bash
cargo test                                    # 79 unit tests — fast, no external deps
cargo test --test challenge_compilation       # 100 rustc compilations — ~9 seconds
```

Test coverage:
- **Engine** (56 tests): language pack deserialization, registry, state machine (9 token sequences), validation (correct/wrong/variants), diff computation, XP calc, module helpers, hint tiers
- **Validator** (23 tests): adapter trait + registry, scaffold splitting, static validation, pack-wide integrity (prompts, hints, explanations, IDs, wrapping)
- **Compilation** (2 integration tests): every challenge + every variant compiles with real `rustc`

## Design system

Custom CSS at `assets/main.css` — "Cosmic Code" dark theme. No Tailwind dependency.

- **Fonts:** Fira Code (code/chips), Sora (UI) via Google Fonts
- **Background:** Deep space (`#07070f`) with subtle star-field radial gradients
- **Chip colors:** 7 categories with neon hues + glow-on-hover (keyword=pink, macro=purple, type=cyan, string=green, symbol=amber, identifier=blue, number=orange)
- **Panels:** Glassmorphism with `backdrop-filter: blur(20px)`
- **All units relative** (rem/em) — zero hardcoded px for font sizes
- **Animation tokens:** CSS custom properties (`--tap: 80ms`, `--panel-slide: 250ms`, etc.)
- **Responsive:** vertical stack by default, two-column grid at 768px+
- **Safe areas:** `env(safe-area-inset-*)` for iOS notch / Android gesture bar

## State management

- **Global state** (`AppState` via `use_context_provider`): XP, streak, progress, language pack
- **Screen state**: component-local `use_signal` for assembled tokens, feedback, hint tier
- **Routing**: `Landing` at `/` redirects to `/onboarding` or `/home` based on `is_onboarded`
- **`NavShell`** renders inside route components (Home, ModuleMap, Profile), not globally

## Platform abstraction (`services/platform.rs`)

Traits for platform-specific APIs — implement per target, no `#[cfg]` in UI code:

- `HapticEngine`: `light_tap()`, `success_pulse()`, `error_tap()`, `double_pulse()`, `medium_tap()`
- `SecureStorage`: `set()`, `get()`, `delete()` — Keychain/Keystore/localStorage

Currently: `NoOpHaptics` and `MemoryStorage` (web stubs).

## Key design principles

1. **No language-specific code in the core app** — everything driven by data (language packs + adapter trait)
2. **Every challenge verified by real compiler** — not just structural checks
3. **Never punishing** — wrong answers say "Not quite", streaks are about showing up
4. **Prompts never leak answers** — describe the goal, not the code
5. **Extensible by data, not code** — new language = JSON file + adapter struct, zero core changes

## Monetization

- No ads ever
- Lifetime: $9.99, Annual: $4.99/yr, Monthly: $1.99/mo
- iOS/Android: In-App Purchase. Web: Stripe checkout (no monthly on web in v1)
- Paywall after Module 3 completion. No dark patterns. Back button always works.

## Supabase

**Project ref:** `dynffktqahqfelfriwfm`

Schema, RLS, and client pattern defined but not yet connected. See `PRD.md` for full database schema.

## Version roadmap

- **v1**: 20 modules, token assembly, streaks, XP, badges, iOS + Android + web
- **v1.5**: Named accounts, Supabase Realtime leaderboard, share URLs, second language pack
- **v2**: WASM on-device compilation, user-generated challenges, community packs
