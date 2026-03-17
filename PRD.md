# TapCode — Product Requirements Document

**Version:** 1.1  
**Date:** March 17, 2026  
**Author:** Draft for review  
**Status:** In progress

---

## 1. Overview

### 1.1 Product summary

TapCode is a mobile-first programming language learning app. Instead of typing code on a tiny keyboard, users build syntactically correct code expressions by tapping token chips organized by semantic category. The experience is closer to solving a puzzle than writing code — deliberate, tactile, and friction-free on a phone.

The platform is designed from the ground up to support multiple programming languages. **Rust is the first language shipped.** Each language is a self-contained curriculum pack with its own token categories, chip colors, compiler adapter, and module tree. Adding a new language requires no changes to the core app — only a new language pack registered in the language registry.

### 1.2 Problem statement

Existing code learning apps (Codecademy, Exercism, LeetCode) assume a keyboard. On mobile, typing `fn main() -> Result<(), Box<dyn Error>>` is punishing. This creates a hard ceiling: mobile can be used for consuming content (videos, articles) but not for practicing code. TapCode removes that ceiling by replacing typing with tapping while preserving the actual syntax and mental models of each language.

Languages with strict, predictable syntax are particularly good fits for this format. Every token has a clear role. Token-based assembly teaches both the grammar and the semantics simultaneously. Rust, Go, TypeScript, and Python are all strong candidates for this reason.

### 1.3 Goals

- Let a complete beginner write real, correct code on their phone within 5 minutes of opening the app
- Ship Rust as the launch language with a complete 10-module curriculum
- Build an extensible language pack architecture so adding language 2 requires zero core app changes
- Achieve a 7-day retention rate of >40% through streaks, XP, and contextual unlocks
- Ship v1 for iOS, Android, and web simultaneously with offline support

### 1.4 Non-goals

- Full on-device compiler in v1 (token sequence matching; v2 adds WASM-based compilation per language)
- Replacing a desktop IDE or full course platform
- Shipping more than one language in v1 (architecture must support it; content does not need to)

---

## 2. Users

### 2.1 Primary persona — "Curious Career Dev"

**Profile:** Software engineer with 2–5 years of experience. Has a language on their list they've been meaning to learn (Rust, Go, a new paradigm). Has tried starting multiple times but lost momentum when the setup overhead or steep learning curve kicked in.  
**Context:** Commutes 30–45 minutes. Has their phone. Does not have their laptop.  
**Goal:** Make real progress on a new language without a laptop, pick things up gradually in stolen time.  
**Frustration:** Every language tutorial assumes a terminal and a full dev environment. Mobile content is passive — no practice.

### 2.2 Secondary persona — "CS Student on the Go"

**Profile:** University student taking a course that uses a language they're not yet fluent in. Needs to practice concepts covered in lectures.  
**Context:** Between classes. Library. Can't always open a laptop.  
**Goal:** Reinforce syntax and concepts before exams. Practice until it feels natural.

### 2.3 Out of scope for v1

- Absolute programming beginners (the curriculum assumes basic familiarity with variables, functions, and types from any language)
- Experts in the language being studied looking for advanced tooling

---

## 3. UX model — Token Assembly

### 3.1 Core mechanic

The user is presented with a code challenge: a natural language description of an expression or program they need to construct in the active language. Below the description is a code canvas showing their current assembly in syntax-highlighted form. Below that is a token picker: a scrollable grid of chips grouped by semantic category.

The user taps chips in order to build the correct token sequence. Each tap appends the token to the canvas. Tapping a token already in the canvas removes all tokens after it (backtrack). A dedicated "undo last" button removes the most recent token.

When the user believes they have the correct sequence, they tap "check". The answer is compared against the canonical token sequence for that challenge. Partial matches show a diff-style highlight. Exact matches trigger XP, animation, and advance to the next challenge.

### 3.2 Token categories

Token categories are semantic roles universal to most languages. Each language pack maps its concrete tokens to these categories and defines its own color ramp. The category names and visual slots in the picker are fixed — only the token contents and color theme change per language.

| Category | Role | Rust example | Python example | Go example |
|---|---|---|---|---|
| Keywords | Language-reserved words | `let`, `fn`, `mut` | `def`, `for`, `with` | `func`, `var`, `defer` |
| Macros / decorators | Language-specific meta constructs | `println!`, `vec!` | `@dataclass`, `@property` | *(not applicable)* |
| Types | Type names and constructors | `i32`, `String`, `Vec<T>` | `int`, `str`, `list` | `int`, `string`, `[]int` |
| String literals | Quoted values | `"Hello"` | `"Hello"` | `"Hello"` |
| Symbols & punctuation | Structural characters | `{`, `}`, `;`, `->` | `:`, `(`, `)` | `{`, `}`, `:=` |
| Identifiers / vars | User-defined names | `x`, `count`, `name` | `x`, `count`, `name` | `x`, `count`, `name` |
| Numbers | Numeric literals | `42`, `3.14` | `42`, `3.14` | `42`, `3.14` |

Color is consistent within a language pack but each language uses a distinct primary hue so switching languages has an obvious visual identity shift.

### 3.3 Contextual chip behavior

As the user builds their expression, the chip picker responds intelligently based on the current parse context defined by the active language pack:

- **After a keyword that implies a name** (e.g. `fn`, `let`, `def`, `func`): the identifier chip group highlights; keyword group dims
- **After `(`**: the type and value groups come forward; symbol group shows `)` prominently
- **After `:`** (or `->` in typed languages): the type group highlights
- **After an opening brace**: a new indented context begins; relevant statement-level chips surface

Contextual rules are defined per language pack as a simple state machine. The core app evaluates the state machine after each tap — no hard-coded language logic in the UI layer.

### 3.4 Hint system

Each challenge has two hint tiers:

- **Tier 1 — concept hint:** A plain-language explanation of the concept being tested, with no code. Example: *"In Rust, variables are immutable by default. Use let to declare one."*
- **Tier 2 — structural hint:** A ghost overlay on the code canvas showing the shape of the answer with blanks. Example: `let _ = _;`

Accessing a hint costs 5 XP. No XP cost for the first hint per session per lesson (encourages new users to explore without fear).

---

## 4. User flows

### 4.0 Design philosophy

Every flow in TapCode is designed around three psychological principles:

**Immediate competence.** The user should feel capable within the first 60 seconds — not overwhelmed, not bored. This means the first challenge is trivially easy, the first dopamine hit comes fast, and difficulty ramps slowly enough that the user never notices they're being stretched.

**Variable reward compression.** The most addictive loops — slot machines, social feeds, loot boxes — compress the gap between action and reward. TapCode applies this to learning: every correct tap is a micro-reward (chip snaps into place), every completed challenge is a small reward (XP burst), every completed module is a large reward (badge unlock, celebration). There is always a reward close in the future.

**Loss aversion as retention.** Streaks work not because gaining a streak feels good, but because *losing* one feels bad. The design uses this carefully: visible streak counts, streak-freeze mechanics, and gentle "don't break it" reminders — but never guilt or shame, which creates churn.

---

### 4.1 Flow 1 — First launch and onboarding

This flow covers the first time a user opens the app. It has one job: get the user to their first correct answer before they have time to question whether this is worth their time. Target: first correct answer within 90 seconds of app open.

```
App opens
    │
    ▼
Splash (500ms)
  TapCode logo animates in. No loading spinner — the WASM bundle
  and all curriculum JSON are already in the binary. Instant.
    │
    ▼
Language select screen
  "What do you want to learn?"
  Cards for available languages. In v1: only Rust, shown large
  and selected. A ghost card with lock icon teases "More languages
  coming soon." One tap, no confirmation needed.
    │
    ▼
Supabase anonymous session created silently in the background.
  No signup form. No email. No friction.
    │
    ▼
Skill check — 2 questions (skippable)
  "Have you written any Rust before?"  [Never]  [A little]  [Yes]
  This routes the user into the curriculum:
    Never    → Module 1, Challenge 1 (absolute beginner entry)
    A little → Module 1, Challenge 3 (skip the trivial hello)
    Yes      → Module 2 or 3 based on a single probe challenge
  Important: this is not a quiz that can be failed. It is routing.
  Users who select "Yes" and then struggle just get gentle hints.
    │
    ▼
Lesson intro card (swipeable, 2 cards max)
  Card 1: "Tap tokens to build code. No typing needed."
  Card 2: A 4-second looping animation showing a chip being
          tapped and snapping into the canvas.
  Skip button visible from card 1. No forced walkthrough.
    │
    ▼
First challenge loads
  Module 1, Challenge 1: Print "Hello, Rust!"
  Prompt: "Print the text Hello, Rust! to the console"
  Chips: extremely limited set — println!, (, "Hello, Rust!", ), ;
  Zero distractors on challenge 1. There is only one possible path.
    │
    ▼
User taps all 5 chips correctly
    │
    ▼
[FIRST DOPAMINE HIT — see animation spec below]
  Chips pulse green in sequence (left to right, 80ms stagger).
  XP counter animates: +20, number bounces.
  "Correct!" feedback panel slides up from bottom.
  A short haptic pulse on mobile (success pattern).
  Confetti particle burst — small, fast, gone in 600ms.
    │
    ▼
"Next →" button on feedback panel.
  User taps next — no auto-advance. Gives them a moment to feel
  the satisfaction before moving on. Auto-advance after 3s if idle.
    │
    ▼
Challenge 2 loads. Difficulty +1 notch. One distractor chip added.
```

**Why this works:** The first challenge is not tutorial content — it is a real Rust expression. The user types real code on their first attempt and it is correct. This creates the "I can actually do this" moment that most programming tutorials withhold for hours.

---

### 4.2 Flow 2 — Core lesson loop

This is the flow the user repeats 5–20 times per session. It is the heartbeat of the product. Every interaction decision here has a direct effect on session length and return rate.

```
Challenge screen loads
    │
    ├─ Prompt text fades in (200ms)
    ├─ Code canvas appears empty with blinking cursor
    ├─ Token picker chips animate in from below (staggered, 30ms each)
    │
    ▼
User reads the prompt
    │
    ▼
User taps first chip
  Chip: scale-down on press (0.92), scale-up on release (1.0), 80ms.
  Token appears in canvas with a brief highlight flash.
  Cursor moves to the right of the new token.
  Chip in the picker dims slightly — visual echo that it was used.
    │
    ▼
User taps subsequent chips
  Each tap: same micro-animation.
  After each tap: contextual highlighting updates (see section 3.3).
  The "check" button becomes active after the first chip is placed.
    │
    ├──────────────────────────────────────────┐
    │  User makes a mistake — taps wrong chip  │
    │                                          ▼
    │                              Wrong chip lands in canvas.
    │                              No immediate punishment.
    │                              User can tap the wrong token
    │                              in the canvas to backtrack to
    │                              that point, or tap "⌫ undo".
    │                              No penalty for self-correction
    │                              before submitting.
    │◄─────────────────────────────────────────┘
    │
    ▼
User taps "Check"
    │
    ├─ Correct ──────────────────────────────────────────────────►
    │                                                            │
    │                                               [DOPAMINE HIT]
    │                                    Tokens pulse green left→right.
    │                                    XP burst animation (+20 or +10).
    │                                    Haptic: success pulse.
    │                                    Feedback panel slides up:
    │                                      "Correct!" + explanation
    │                                      of what the code does.
    │                                    Progress bar fills a notch.
    │                                    "Next →" button.
    │                                    Auto-advance after 3s.
    │
    └─ Wrong ────────────────────────────────────────────────────►
                                                                 │
                                             Tokens show diff highlight:
                                               Green = correct position
                                               Red   = wrong token
                                               Ghost = missing token
                                             Feedback panel slides up:
                                               "Not quite" (never "Wrong")
                                               Brief explanation of the
                                               correct concept.
                                             "Try again" button — panel
                                             dismisses, canvas resets to
                                             the user's last attempt with
                                             wrong tokens highlighted.
                                             Retry awards 10 XP on correct.
                                             No streak penalty for wrong
                                             answers. Streaks are about
                                             showing up, not perfection.
```

**Tone on failure:** The copy never says "Wrong", "Incorrect", or "Failed". It says "Not quite", "Almost", or "Close — here's the difference". The diff highlight shows the user exactly where they went wrong without rubbing it in. The goal is for a wrong answer to feel like information, not judgment.

**Explanation on correct answers:** The feedback panel for a correct answer always includes one sentence explaining what the code does or why it is written that way. Example: *"println! is a macro — note the !. Macros in Rust are expanded at compile time, not called like functions."* This turns a pattern-matching exercise into actual learning.

---

### 4.3 Flow 3 — Module completion

Triggered when the user completes the last challenge in a module. This is the largest reward moment in the app and must feel earned and significant without being annoying.

```
Last challenge in module answered correctly
    │
    ▼
Normal correct-answer animation plays (tokens go green, XP burst)
    │
    ▼
Feedback panel shows briefly, then transitions to:

Module completion full-screen moment (not a modal — full screen)
  ┌─────────────────────────────────────┐
  │                                     │
  │   [Badge animation — 1.5s]         │
  │   Badge drops in from top,          │
  │   rotates once, settles.            │
  │   Particle burst behind it.         │
  │                                     │
  │   "Module 2 complete"               │
  │   Variables and Bindings            │
  │                                     │
  │   [XP total for this module: +120]  │
  │   Number counts up from 0.          │
  │                                     │
  │   [Accuracy: 87%]                   │
  │   [Challenges: 5  •  Hints used: 1] │
  │                                     │
  │   [Unlock: Free Compose — Variables]│
  │   Small preview of the compose mode.│
  │                                     │
  │   ─────────────────────────────     │
  │   Next up: Module 3                 │
  │   Functions                         │
  │   [5 challenges · ~8 min]           │
  │                                     │
  │   [Start Module 3]   [Go Home]      │
  └─────────────────────────────────────┘

Haptic: double pulse — more emphatic than a single challenge.
```

**Why full-screen:** A modal says "this is a notification". A full-screen moment says "this is an event". The difference in felt significance is large. The user has just learned something real — the app matches that weight.

**"Next up" preview:** Showing the next module title and time estimate at the completion screen significantly increases immediate continuation rate. The user's momentum is at its peak right after a module complete — capturing that momentum is critical.

---

### 4.4 Flow 4 — Daily return / streak maintenance

The most important retention flow. The user opens the app to maintain their streak. This must be fast enough that doing it feels effortless, and rewarding enough that missing it feels like a real loss.

```
User opens app (returning, has streak)
    │
    ▼
Home screen
  Streak counter is the dominant visual element.
  "🔥 7" in large type. Yesterday's date highlighted in the
  streak calendar row. Today's slot is empty with a pulsing ring.
  The ring communicates: "one challenge fills this".
    │
    ▼
"Continue" button is the primary CTA — goes straight to
  the next unsolved challenge. No menu navigation needed.
  Secondary: browse modules, free compose, settings.
    │
    ▼
User taps Continue → Challenge loads immediately
    │
    ▼
User completes 1 challenge (minimum streak requirement)
    │
    ▼
[STREAK FILL MOMENT]
  Today's slot in the streak calendar fills in with an orange glow.
  The streak number increments with a bounce animation.
  Haptic: satisfying single tap.
  Copy: "Streak kept! 🔥 8 days"
  This moment is distinct from the challenge-correct animation —
  it is calmer, more earned-feeling. Not a burst. A glow.
    │
    ▼
User can stop here — streak is safe — or continue for more XP.
  No guilt if they stop. No "are you sure you want to quit?" dialog.
```

**Streak architecture (UX implications):**

The streak calendar shows the last 7 days as a horizontal row of dots. Completed days are filled orange. Today's empty slot has the pulsing ring. A small "🧊 freeze available" icon appears below when a freeze is banked — this communicates the safety net exists without emphasizing it so much that it reduces urgency.

**Streak loss flow:** If the user opens the app after a missed day (streak broken):

```
Home screen — streak is 0 (or reset)
  Previous streak is shown in muted type: "Previous streak: 8 days"
  No dramatic animation. No shaming copy.
  Copy: "Start a new streak today."
  The rebuild narrative is positive. The loss is acknowledged
  but not dwelt on. The CTA is identical: "Continue →"
```

**Push notification (opt-in):** Sent at 7pm if the user has not opened the app that day. Copy variants: "Your 8-day streak is waiting." / "One challenge. That's all." / "Rust won't learn itself." Notification is skipped if the user has already completed a challenge that day.

---

### 4.5 Flow 5 — Hint usage

The hint system must feel helpful, not shameful. A user who reaches for a hint is at the edge of their knowledge — exactly where learning happens. The flow should make using a hint feel like a smart move, not a failure.

```
User is stuck on a challenge
    │
    ▼
"Hint ?" button (always visible, bottom left of action bar)
    │
    ▼
Tier 1 hint — concept explanation
  A card slides up from the bottom (not full-screen — partial).
  Plain English explanation of the relevant concept.
  No code. No answer.
  Small "-5 XP" label in the corner. Visible but not alarming.
  "Got it" button dismisses the card.
    │
    ▼
User tries again. Still stuck.
    │
    ▼
"Hint ?" tapped again on same challenge
    │
    ▼
Tier 2 hint — structural ghost
  The code canvas shows a ghost overlay:
    let _ = _;
  User's already-placed tokens remain; the ghost fills the
  remaining shape. This shows structure without giving away tokens.
  Same -5 XP cost.
    │
    ▼
User taps "Hint ?" a third time (only if still stuck)
    │
    ▼
"Skip challenge" option appears alongside hint.
  Copy: "Still stuck? Skip this one and come back later."
  Skipping costs no XP and does not break streaks.
  The challenge is added to a "revisit" queue shown on the home
  screen as a gentle nudge, never a requirement.
```

**Why skip exists:** Frustration is the #1 reason users churn from learning apps. A stuck user who cannot skip will quit the app. A stuck user who skips will come back. The revisit queue ensures the skipped challenge is not forgotten.

---

### 4.6 Flow 6 — Paywall encounter

The paywall is not a wall. It is a transition — from free to valued. The user must already believe TapCode is worth paying for before they see the price. The design achieves this by placing the paywall at the end of Module 3 (not before), after the user has had enough time to feel genuine progress and habit formation.

```
User completes Module 3 (Functions) — the last free module
    │
    ▼
Module 3 completion screen plays in full (badge, XP, celebration).
  This is intentional: the reward for completing the free content
  is delivered in full before any purchase mention.
    │
    ▼
After the celebration settles (2s delay):

Paywall screen — not a modal, a full screen with back navigation
  ┌─────────────────────────────────────┐
  │  Keep going with Rust               │
  │                                     │
  │  You've learned:                    │
  │  ✓ Output                           │
  │  ✓ Variables & bindings             │
  │  ✓ Functions                        │
  │                                     │
  │  Unlocks next:                      │
  │  → Control flow                     │
  │  → Ownership (the hard part)        │
  │  → Structs, Enums, Error handling   │
  │  → Traits and Collections           │
  │  + All future languages             │
  │                                     │
  │  [Annual — €4.99/yr]  ← highlighted │
  │  [Lifetime — €9.99]                 │
  │  [Monthly — €1.99/mo]               │
  │                                     │
  │  ──────────────────────────────     │
  │  Already purchased? Restore         │
  │  Questions? See FAQ                 │
  └─────────────────────────────────────┘

No dark patterns:
  - No countdown timer
  - No "limited offer" language
  - No guilt-trip copy ("Are you sure you want to give up?")
  - Back button works. User can revisit free modules freely.
```

**Web paywall variant:** On web, the same screen appears but the purchase buttons trigger a Stripe checkout session (redirect flow). The annual and lifetime options are identical. Monthly is not offered on web in v1 to reduce the complexity of the Stripe integration.

---

### 4.7 Flow 7 — Free compose mode

Unlocked per module. This is the "playground" — no right answer, just syntax validation. It serves a different psychological need: mastery and autonomy, not guided learning.

```
User navigates to Free Compose (via module badge → "Compose")
    │
    ▼
Compose screen
  Same layout as the lesson screen but:
  - No prompt text at the top
  - No "check" button — replaced with "validate" which checks
    whether the current assembly is syntactically valid (not correct
    against an answer, just valid Rust/Go/etc.)
  - All chips from the unlocked module's concept space are available
  - A "clear" button resets the canvas
  - A "share" button (v1.5) generates a tapcode.dev/compose/... URL
    with the current assembly encoded in the path
    │
    ▼
User taps "Validate"
    │
    ├─ Valid syntax
    │     Canvas border glows green briefly.
    │     Copy: "Valid ✓" in the top right corner. No fanfare.
    │     This is a sandbox. Celebration is calibrated accordingly.
    │
    └─ Invalid syntax
          A subtle red pulse. Brief message:
          "Not valid — missing semicolon?" (heuristic error hint)
          No XP cost. No penalty. This is a playground.
```

---

### 4.8 Micro-interaction and animation specification

Every animation serves a function. None are decorative. Timings are chosen to match the natural rhythm of tapping.

| Moment | Animation | Duration | Haptic |
|---|---|---|---|
| Chip tap | Scale 1.0 → 0.92 → 1.0 | 80ms | Light tap (iOS: `.light`) |
| Token appears in canvas | Slide in from right + fade | 120ms | None |
| Undo last token | Token slides out right + fade | 100ms | None |
| Correct answer | Tokens pulse green left→right (80ms stagger) | 480ms total | Success pulse |
| XP increment | Number bounces + glow | 300ms | None |
| Confetti burst | 20 particles, radial, fade out | 600ms | None |
| Wrong answer diff | Tokens flash red/green simultaneously | 300ms | Error tap (iOS: `.error`) |
| Feedback panel slide-up | Slide from bottom, ease-out | 250ms | None |
| Module badge drop | Drop in + rotation + settle | 1500ms | Double pulse |
| Streak fill | Dot glows orange, number bounces | 400ms | Single medium tap |
| Progress bar fill | Ease-in-out fill | 300ms | None |
| Chip group contextual highlight | Opacity 1.0 → 0.4 (dimmed groups) | 150ms | None |

**Sound design (opt-in, off by default):**

| Moment | Sound |
|---|---|
| Chip tap | Soft click (wood-like, not plastic) |
| Token snaps into canvas | Subtle magnetic snap |
| Correct answer | Rising two-note chord (major second) |
| Wrong answer | Single low soft thud |
| Module complete | Four-note ascending melody |
| Streak fill | Single resonant ping |

Sound is off by default and toggled in settings. First time sound would play (challenge 1 correct), a tooltip appears: "Want sounds? Enable in settings."

---

### 4.9 Empty and error states

**No internet on first launch (web):**  
"TapCode needs a connection to get started. Once loaded, you can use it offline." — retry button.

**No internet on returning launch (all platforms):**  
App works fully. A small banner: "Offline — progress will sync when you reconnect." No blocking dialog. Learning continues.

**Supabase write fails (offline queue can't flush):**  
Silent retry. Never shown to the user unless the queue has been building for >24 hours, at which point a soft notification: "Some progress hasn't synced yet. We'll retry automatically."

**All modules completed (no more content):**  
"You've completed all available Rust modules." + progress stats + a teaser for upcoming modules with a notify-me button. Never a dead end.

**Session too long (>45 minutes active):**  
A gentle interstitial: "Great session — you've been going for 45 minutes. Taking a break helps retention." Dismissable immediately. Never reappears in the same session.

---

## 5. Language packs and curriculum

### 5.1 Language pack concept

A language pack is a self-contained bundle that gives TapCode everything it needs to teach a language. The core app has no language-specific logic. It is a generic token assembly engine that consumes a language pack at runtime.

Each language pack contains:

- **Metadata:** language ID, display name, version, primary color hue, icon, tagline
- **Token category definitions:** the concrete tokens for each category (keywords, types, symbols, etc.) with their display labels and chip colors
- **Contextual state machine:** rules for which chip groups to highlight after which token sequences
- **Syntax highlighter config:** token-to-color mappings for the code canvas
- **Module tree:** ordered list of modules, each containing an ordered list of challenge IDs
- **Challenge bundle:** all challenge JSON objects for this language (see section 8 schema)
- **Compiler adapter config:** how the validation pipeline reconstructs and runs full programs from challenge fragments (see section 7)

Language packs are versioned independently of the app. A new language pack can be shipped as an OTA content update. The app renders whatever packs are available in the registry.

### 5.2 Language registry

The app maintains a language registry — a list of available language packs. On the home screen, the user picks their active language. In v1, only Rust is in the registry. In v1.5, a second language (candidate: Go or Python) is added. In v2, the registry is opened to community-authored packs.

```json
{
  "languages": [
    {
      "id": "rust",
      "display_name": "Rust",
      "version": "1.0.0",
      "status": "available",
      "primary_hue": "#f5a623",
      "icon": "ferris",
      "tagline": "Systems programming. Memory safe."
    },
    {
      "id": "go",
      "display_name": "Go",
      "version": "0.1.0",
      "status": "coming_soon"
    }
  ]
}
```

### 5.3 Curriculum structure

Within each language pack, the curriculum follows the same structural rules:

- Lessons are grouped into **modules** of 5–10 challenges each
- Modules unlock sequentially
- Challenges within a module are ordered from simplest token count to most complex
- Completing a module awards a language-specific badge and unlocks free compose mode for that module's concept space

### 5.4 Rust curriculum (v1 language pack)

**Module 1 — First output**
- Print a static string with `println!`
- Print with a format argument: `println!("{}", x)`
- Print two arguments
- Use `print!` (no newline)
- Use `dbg!` on a value

**Module 2 — Variables and bindings**
- Declare an immutable variable: `let x = 42;`
- Declare a mutable variable: `let mut x = 0;`
- Reassign a mutable variable: `x = 10;`
- Declare with explicit type: `let x: i32 = 5;`
- Shadow a variable: declare `x`, then declare `x` again with a new value

**Module 3 — Functions**
- Define a zero-argument, no-return function
- Define a function with one parameter and its type
- Define a function with a return type: `fn add(a: i32, b: i32) -> i32`
- Write the function body with an implicit return (no semicolon on last expression)
- Call a function

**Module 4 — Control flow**
- Write an `if`/`else` block
- Write a `loop {}` with a `break`
- Write a `for x in 0..5 {}` loop
- Write a `while` loop
- Write a `match` expression with two arms

**Module 5 — Ownership basics**
- Create a `String` with `String::new()`
- Create a `String` from a literal with `String::from("...")`
- Move a `String` into a function call (understand move semantics visually)
- Clone a `String`: `s.clone()`
- Borrow a `String`: pass `&s` to a function

**Module 6 — Structs**
- Define a struct with two fields
- Instantiate a struct
- Access a struct field with `.`
- Implement a method on a struct with `impl`
- Implement a `new()` constructor

**Module 7 — Enums and match**
- Define an enum with two variants
- Define an enum variant that holds data
- Match on an enum
- Use `Option<T>`: `Some(x)` and `None`
- Use `Result<T, E>`: `Ok(x)` and `Err(e)`

**Module 8 — Error handling**
- Unwrap a `Result`: `.unwrap()`
- Use `expect` with a message: `.expect("msg")`
- Use the `?` operator in a function returning `Result`
- Write a `match` on `Result` with `Ok` and `Err` arms
- Return a `Result` from `main`

**Module 9 — Traits (intro)**
- Derive `Debug` on a struct
- Implement `Display` for a struct
- Use a generic function: `fn largest<T: PartialOrd>(list: &[T])`
- Implement a custom trait with one method
- Use `impl Trait` in a function argument

**Module 10 — Collections**
- Create a `Vec<i32>` with `vec![]`
- Push to a Vec: `.push(x)`
- Iterate over a Vec with `for`
- Create a `HashMap` and insert a key-value pair
- Look up a value in a `HashMap` with `.get()`

### 5.5 Rust post-v1 modules (backlog)

- Closures and iterators (`map`, `filter`, `collect`)
- Lifetimes
- Trait objects and dynamic dispatch
- Async/await with Tokio
- Unsafe Rust

### 5.6 Candidate future language packs

| Language | Why it fits | Key differentiator vs Rust pack |
|---|---|---|
| Go | Simple syntax, strict formatting, high demand | Different concurrency model; goroutines as a late module |
| Python | Whitespace-significant; teaches indentation as syntax | Decorator and comprehension chips |
| TypeScript | Familiar to large audience; type system as teaching hook | Generic types, union types as chip categories |
| Zig | Systems language, comptime as novel concept | Comptime chips as a distinct category |
| SQL | Not general-purpose but highest demand for mobile practice | Clause ordering as the core mechanic |

---

## 6. Progression and engagement

### 6.1 XP and leveling

Every correct first-attempt answer awards 20 XP. Subsequent retries after an incorrect attempt award 10 XP. XP is cumulative and displayed on the profile. Levels are defined at XP thresholds: level 2 at 100 XP, level 3 at 300 XP, doubling each level.

### 6.2 Streaks

A daily streak is maintained if the user completes at least one challenge per calendar day. Streaks are shown prominently on the home screen. At 7-day and 30-day streak milestones, a special animation plays and a badge is awarded.

Streak freeze: the user can bank one streak freeze per week (awarded automatically). A streak freeze automatically activates on the first missed day. Second missed day breaks the streak.

### 6.3 Module badges

Each completed module awards a badge. Badge artwork is defined per language pack — Rust badges are themed around Ferris the crab and oxidized metal; future language packs define their own visual language. Badges are displayed on the profile screen grouped by language. Badges are not locked behind performance — completing a module always awards its badge regardless of number of hints or retries.

### 6.4 Challenge difficulty scaling

Within a module, difficulty scales along three axes:

- **Token count:** challenges start with 3–4 tokens and scale to 12–15 tokens in later challenges
- **Distractor chips:** early challenges have 0–1 decoy chips per group; later challenges have 2–3 per group
- **Contextual help:** early challenges have contextual chip highlighting enabled by default; it can be disabled in settings for a harder mode

### 6.5 Free compose mode

Unlocked per module upon module completion. The user gets a blank canvas with all chips from that module's concept space available. No correct answer is expected. The system performs only syntactic validation against the active language pack's grammar rules (does this form a valid expression/statement?). Useful for experimentation and review.

---

## 7. Answer validation

### 7.1 v1 — token sequence matching

In v1, each challenge has a canonical answer stored as an ordered array of token strings. The user's assembled token sequence is compared against it exactly. Minor variants are normalized before comparison because tokens are discrete — spacing is irrelevant.

Multiple canonical answers can be defined per challenge where a language genuinely allows semantic equivalence (e.g. Rust implicit vs explicit return, Python `f""` vs `.format()`).

### 7.2 v2 — WASM compilation (future)

In v2, the assembled token sequence is fed to the language's compiler or interpreter compiled to WASM running on-device. The output is compared against expected stdout or evaluated for compilation/parse success. This enables open-ended challenges without a single canonical answer. Each language pack in v2 ships a WASM compiler adapter alongside its content.

### 7.3 Feedback on wrong answers

When the answer is wrong, the system shows a diff between the user's sequence and the closest canonical answer:
- Correct tokens in the right position: shown in green
- Wrong token in a position: shown in red with the correct token as a ghost hint
- Extra tokens: flagged with a removal indicator
- Missing tokens: shown as empty ghost slots

The user taps on a wrong token to remove everything from that point and re-assemble.

---

## 8. Challenge validation pipeline

Before any challenge is published to the app, its canonical answer(s) must be verified against a real compiler or interpreter for that language. This guarantees that every challenge users see is provably correct — the app never ships a challenge whose answer does not compile/run and produce the expected output.

### 8.1 Rationale

Token sequence matching (section 6.1) is fast and works offline, but it cannot catch mistakes made during challenge authoring. A misplaced semicolon, a wrong type, a missing `&` — these are easy to introduce when writing challenges in a CMS. Without compiler verification, bad challenges reach users, erode trust, and generate support noise. The validation pipeline is the quality gate that prevents this entirely.

### 8.2 Architecture overview

The pipeline is language-agnostic at the orchestration layer. Language-specific logic lives entirely in **compiler adapters** — one per language, each a thin Docker image that knows how to reconstruct a runnable program from a challenge fragment and invoke the language toolchain.

```
CMS (challenge authored)
        │
        ▼
Webhook → Validation Orchestrator (language-agnostic)
        │
        ├─ Reads challenge.language → selects compiler adapter
        │
        ├─ Adapter: rust    → rustc / cargo check
        ├─ Adapter: go      → go run
        ├─ Adapter: python  → python3
        ├─ Adapter: ...     → (extensible)
        │
        ├─ Reconstructs full program from fragment + scaffolding
        ├─ Runs in sandboxed container
        ├─ Compares stdout / stderr / exit code against expected
        │
        ├─ PASS → challenge marked verified, eligible for publish
        └─ FAIL → challenge blocked, author notified with toolchain output
```

Adding a new language requires only a new compiler adapter Docker image. The orchestrator, CMS integration, CI pipeline, and schema are unchanged.

### 8.3 Compiler adapter interface

Each compiler adapter implements a standard interface:

```json
{
  "adapter_id": "rust",
  "image": "tapcode-validator-rust:1.77.0",
  "fragment_wrappers": {
    "expression":  "fn main() { let _ = {FRAGMENT}; }",
    "statement":   "fn main() { {FRAGMENT} }",
    "fn_def":      "{FRAGMENT}\nfn main() {}",
    "type_def":    "{FRAGMENT}\nfn main() {}",
    "program":     "{FRAGMENT}"
  },
  "run_command": "rustc /tmp/challenge.rs -o /tmp/out && /tmp/out",
  "timeout_seconds": 5
}
```

```json
{
  "adapter_id": "python",
  "image": "tapcode-validator-python:3.12",
  "fragment_wrappers": {
    "expression":  "_ = {FRAGMENT}",
    "statement":   "{FRAGMENT}",
    "fn_def":      "{FRAGMENT}",
    "program":     "{FRAGMENT}"
  },
  "run_command": "python3 /tmp/challenge.py",
  "timeout_seconds": 5
}
```

The challenge schema's `language` field selects the adapter. No other part of the pipeline changes.

### 8.4 Expected output contract

```json
{
  "id": "rust-m1-c1",
  "language": "rust",
  "fragment_type": "statement",
  "answer": ["println!", "(", "\"Hello, Rust!\"", ")", ";"],
  "expected": {
    "exit_code": 0,
    "stdout": "Hello, Rust!\n",
    "stderr": ""
  }
}
```

For challenges where the correct answer produces no output (e.g. a struct definition), `stdout` is an empty string and only `exit_code: 0` is asserted. For challenges testing that something intentionally does not compile (future advanced modules), `exit_code` is non-zero and a `stderr_contains` substring match is used instead.

### 8.5 Multi-variant validation

When a challenge defines `answer_variants` (semantically equivalent correct answers), every variant is run through the validator independently. A challenge is only marked verified if all variants pass. Any variant that fails is either fixed or removed before publish.

### 8.6 Sandbox

Every adapter runs inside a Docker container with:

- No network access
- Read-only filesystem except `/tmp`
- 5-second execution timeout
- Memory limit: 512 MB
- Single-use containers — a fresh container per validation run, discarded immediately after

This prevents any possibility of authored challenge code affecting the host or other validation runs. The same sandbox constraints apply regardless of language.

### 8.7 CI integration

Every pull request that modifies any challenge JSON file or compiler adapter config triggers the validator for all changed challenges. PRs cannot be merged if any challenge fails validation. This applies to:

- New challenges (any language)
- Edits to `answer` or `answer_variants`
- Edits to `expected` output
- Edits to `fragment_type` or `language`
- Edits to a compiler adapter image or wrapper templates

Changes to non-semantic fields (prompt text, hint text, chip ordering) skip the compiler run and pass automatically.

### 8.8 CMS webhook (on-demand)

Challenge authors work in a headless CMS. When an author clicks "Submit for review", the CMS fires a webhook to the validation orchestrator. The result (pass/fail + toolchain output) is written back to the CMS record within ~10 seconds and displayed inline to the author. A challenge in "failed" state cannot be moved to "approved" or "published" status in the CMS workflow.

### 8.9 Validation status field in challenge schema

```json
{
  "validation": {
    "status": "verified",
    "last_run": "2026-03-17T09:41:00Z",
    "adapter_id": "rust",
    "toolchain_version": "rustc 1.77.0",
    "variants_verified": 1,
    "variants_total": 1
  }
}
```

The app bundle build pipeline rejects any challenge where `status` is not `"verified"`. Challenges can also have `status: "pending"` (not yet run) or `status: "failed"` (toolchain rejected).

---

## 9. Content management

### 9.1 Challenge schema

Each challenge is defined as a JSON object. The `language` field binds it to a language pack and selects the compiler adapter used during validation.

```json
{
  "id": "rust-m2-c3",
  "language": "rust",
  "module": 2,
  "position": 3,
  "title": "Reassign a mutable variable",
  "prompt": "Reassign the variable x to the value 10",
  "hint_concept": "You can only reassign a variable declared with mut. Use = to assign a new value.",
  "hint_structural": "_ = _;",
  "fragment_type": "statement",
  "answer": ["x", "=", "10", ";"],
  "answer_variants": [],
  "expected": {
    "exit_code": 0,
    "stdout": "",
    "stderr": ""
  },
  "chips": [
    { "group": "identifiers", "tokens": ["x", "y", "count"] },
    { "group": "values",      "tokens": ["10", "0", "42"] },
    { "group": "symbols",     "tokens": ["=", ";", ":"] },
    { "group": "keywords",    "tokens": ["let", "mut"] }
  ],
  "xp": 20,
  "validation": {
    "status": "verified",
    "last_run": "2026-03-17T09:41:00Z",
    "adapter_id": "rust",
    "toolchain_version": "rustc 1.77.0",
    "variants_verified": 1,
    "variants_total": 1
  }
}
```

Challenge IDs are namespaced by language (`rust-m2-c3`, `go-m1-c1`) to prevent collisions across language packs.

### 9.2 Content pipeline

Challenges are authored in a private CMS per language pack and exported as JSON bundles. The app ships with all available language packs embedded. Updates to challenges and new modules are delivered as OTA content updates (no app store update required). New language packs are delivered as app updates in v1; in v2 they may be dynamically downloadable.

---

## 10. Technical architecture

### 10.1 Framework — Dioxus

TapCode is built with [Dioxus](https://github.com/DioxusLabs/dioxus), a fullstack Rust UI framework that compiles to a single native binary for iOS, Android, web, and desktop from one codebase. The core app is language-agnostic — it is a generic token assembly engine and language pack renderer. Language packs are data, not code; no Rust recompile is needed to add a new language.

Dioxus uses RSX, a JSX-inspired macro for declaring UI in Rust. Components are pure Rust functions. State is managed via a fine-grained signals system modelled after SolidJS and Svelte — reactive primitives (`Signal<T>`, `Memo<T>`) that update only the components that depend on them.

```toml
[dependencies]
dioxus  = { version = "0.6", features = ["router", "fullstack"] }
reqwest = { version = "0.12", features = ["json", "wasm"] }
serde   = { version = "1",   features = ["derive"] }
serde_json = { version = "1" }
tokio   = { version = "1",   features = ["full"], optional = true }

[features]
default = []
web     = ["dioxus/web"]
desktop = ["dioxus/desktop"]
mobile  = ["dioxus/mobile"]
server  = ["dioxus/server", "dep:tokio"]
```

### 10.2 Platform targets

| Platform | Feature flag | Command | v1? | Notes |
|---|---|---|---|---|
| iOS | `mobile` | `dx serve --platform ios` | Yes | Primary mobile target |
| Android | `mobile` | `dx serve --platform android` | Yes | Functional; requires NDK setup — allocate extra time |
| Web | `web` | `dx serve --platform web` | Yes | WASM output; see section 9.12 |
| Desktop | `desktop` | `dx serve --platform desktop` | No | Available for free given shared codebase; not actively shipped |

Production builds via `dx bundle`. Mobile binaries are under 5 MB. Web output is WASM + HTML + CSS, served as a static site — no server required for the app itself.

### 10.3 Storage — Supabase

All persistent state (user progress, XP, streaks, completed challenges, unlocked modules, language progress) is stored in **Supabase** (hosted Postgres). Supabase is the single source of truth across all platforms — iOS, Android, and web all read and write to the same backend with no platform-specific storage layer.

**Authentication.** Supabase Auth handles identity. On first launch the app creates an anonymous session automatically — no signup form, no friction. The anonymous session is tied to the device via a persisted JWT stored in the platform's secure storage (Keychain on iOS, Keystore on Android, `localStorage` on web). The user can optionally upgrade to a named account (email or OAuth) at any point, which merges the anonymous progress into the named account.

**Database schema.**

```sql
-- one row per user
create table user_state (
  id            uuid primary key references auth.users,
  created_at    timestamptz default now(),
  total_xp      int default 0,
  current_streak int default 0,
  longest_streak int default 0,
  last_active   date
);

-- per-language progress
create table language_progress (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid references user_state,
  language_id text not null,
  xp          int default 0,
  active_module int default 1,
  unlocked_modules int[] default '{1}',
  unique (user_id, language_id)
);

-- every attempt at a challenge
create table challenge_attempts (
  id           uuid primary key default gen_random_uuid(),
  user_id      uuid references user_state,
  challenge_id text not null,          -- e.g. "rust-m2-c3"
  language_id  text not null,
  correct      boolean not null,
  attempt_num  int not null,           -- 1 = first try
  attempted_at timestamptz default now()
);

-- one row per calendar day the user was active
create table streak_log (
  user_id uuid references user_state,
  day     date not null,
  primary key (user_id, day)
);
```

Row-Level Security (RLS) is enabled on all tables. Every policy is `user_id = auth.uid()` — users can only read and write their own rows. No privileged server process is needed for normal app operation.

**Client.** The app communicates with Supabase via its REST API (PostgREST) and Auth REST endpoints using `reqwest` with the `wasm` feature, which compiles correctly for both native and WASM targets. A thin `SupabaseClient` struct wraps `reqwest`, injects the `apikey` and `Authorization: Bearer <jwt>` headers, and exposes typed methods for each table operation.

```rust
struct SupabaseClient {
    base_url: String,
    anon_key: String,
    jwt: Signal<Option<String>>,
    http: reqwest::Client,
}

impl SupabaseClient {
    async fn upsert_language_progress(&self, p: &LanguageProgress)
        -> Result<(), AppError> { ... }

    async fn record_attempt(&self, a: &ChallengeAttempt)
        -> Result<(), AppError> { ... }

    async fn fetch_user_state(&self)
        -> Result<UserState, AppError> { ... }
}
```

**Optimistic updates.** To keep the UI instant, every mutation is applied to the local Dioxus signal state immediately — before the Supabase call returns. The network call happens in a background `use_coroutine`. On failure, the signal is rolled back and a toast is shown. This means the app feels fully responsive even on slow connections.

**Offline queue.** When the device has no network, mutations are pushed to an in-memory queue (`Signal<Vec<PendingMutation>>`). A background coroutine polls for connectivity and flushes the queue when the connection is restored. On mobile, the queue is also serialized to a local file so it survives app restarts while offline. On web, it is serialized to `localStorage`. The queue is small in practice — challenge completions and XP increments are the only frequent writes.

### 10.4 State management

Dioxus signals handle all in-memory UI state. The signal graph for the core lesson screen:

```rust
let assembled_tokens: Signal<Vec<String>>  = use_signal(|| vec![]);
let current_lesson:   Signal<usize>        = use_signal(|| 0);
let xp:               Signal<u32>          = use_context::<Signal<u32>>();
let feedback_state:   Signal<FeedbackKind> = use_signal(|| FeedbackKind::None);
```

Global app state (XP, streak, unlocked modules) is provided via `use_context` from a root provider component. On every mutation, the signal is updated immediately (optimistic) and the change is written to Supabase asynchronously via the offline queue (see section 9.3). Chip selection, animation triggers, and transient UI state are component-local signals.

### 10.5 Routing

Dioxus Router (`dioxus/router`) handles navigation between screens. Route enum:

```rust
#[derive(Routable, Clone)]
enum Route {
    #[route("/")]          Home {},
    #[route("/lesson/:id")] Lesson { id: String },
    #[route("/module/:id")] ModuleMap { id: String },
    #[route("/profile")]   Profile {},
    #[route("/compose/:module_id")] FreeCompose { module_id: String },
}
```

### 10.6 Styling

TailwindCSS via Dioxus's built-in Tailwind support (`dx` CLI handles the build step). All token chip colors, code canvas colors, and feedback states are defined as Tailwind utility classes applied inside RSX. A custom Tailwind config defines the RusTap color palette (dark background, syntax-highlighting ramps) as design tokens.

### 10.7 Animations

CSS transitions are used for chip tap feedback (scale transform), feedback panel slide-in, and progress bar fill. For the correct-answer celebration (particle burst), a small Rust animation loop drives frame-by-frame updates via a `use_coroutine` that updates a `Signal<Vec<Particle>>` at 60fps. All animations target ≤300ms to feel native.

### 10.8 Backend — Supabase project setup

TapCode uses a single Supabase project for all environments (separate projects for staging and production). No custom server process is needed for normal app operation — Supabase's auto-generated PostgREST API, Auth, and RLS policies handle everything.

**Project configuration checklist:**

- Enable Email auth and Anonymous sign-in in the Supabase Auth settings
- Apply the schema migrations from `supabase/migrations/` via the Supabase CLI (`supabase db push`)
- Enable RLS on all tables; apply the `user_id = auth.uid()` policies
- Set `JWT_SECRET`, `SUPABASE_URL`, and `SUPABASE_ANON_KEY` as environment variables baked into the app at build time via the `dx` CLI asset pipeline

**Dioxus server functions.** The `server` feature flag is used only for the optional CMS sync webhook and the challenge validation pipeline orchestrator — not for normal user-facing app traffic. All user data flows directly from the app to Supabase. There is no custom API server in the critical path.

**Realtime (v1.5).** Supabase Realtime can be used in v1.5 to power the opt-in global leaderboard — subscribing to `language_progress` aggregate changes without polling. Not required in v1.

### 10.9 Offline support

All curriculum JSON is compiled into the binary as a static asset (via `include_str!` or the Dioxus asset pipeline). The core learning loop — reading challenges, tapping chips, getting feedback — requires no network access.

Writes (challenge completions, XP increments, streak updates) are handled via the offline queue described in section 9.3. The UI reflects the optimistic local state immediately. When connectivity is restored the queue flushes to Supabase automatically. Conflicts are resolved with a last-write-wins policy on `challenge_attempts` (append-only) and a max-value policy on XP and streak fields.

### 10.10 Known Dioxus mobile constraints

- Android setup requires NDK configuration and cross-compilation toolchain (`cargo-ndk`). Allocate a full day of environment setup per developer.
- Hot-patching (`dx serve --hotpatch`) for Rust code changes is experimental; RSX/CSS hot-reload is stable and fast.
- Some native mobile APIs (haptics, system fonts, safe area insets) require calling into JNI (Android) or Objective-C (iOS) directly. Wrap these behind a platform abstraction trait so the web/desktop builds compile without them.
- Test on real devices early. The simulator/emulator experience for Dioxus mobile is good; real device performance testing should begin at the end of module 2 implementation, not at launch.

---

### 10.11 Accessibility

- All tap target chip elements include `aria_label` attributes in RSX for VoiceOver and TalkBack
- Font size follows system dynamic type — use relative units, not hardcoded px
- Color is never the sole indicator of state — all feedback uses both color and an icon or text label
- VoiceOver and TalkBack full traversal test required before App Store submission

### 10.12 Web platform

Dioxus compiles to WASM for the web via `dx bundle --platform web`, producing a static site bundle (HTML + WASM + CSS) that can be hosted on any CDN with no server-side runtime. This is the lowest-friction distribution channel — no app store, no install, accessible from any link.

**URL-based deep linking.** The web build enables shareable challenge URLs: `tapcode.dev/rust/m2/c3` routes the user directly into a specific challenge. This is the primary viral growth mechanic for web — a user can share a challenge link on social media, Slack, or a blog post. Deep linking is handled by Dioxus Router; the route enum is identical across all platforms with web-specific URL rendering.

**Keyboard support.** On web, users arrive on desktops and laptops as well as phones. The tap mechanic maps naturally to click, but keyboard navigation must also work: number keys 1–9 select chips within the focused group, Enter submits, Backspace undoes the last token, and Tab moves between chip groups. This is a web-only enhancement — not required on mobile builds.

**Responsive layout.** The mobile layout (vertical stack: prompt → canvas → picker → action bar) is the base. On viewports wider than 768px, the layout shifts to a two-column split: prompt and canvas on the left, token picker on the right. This makes the web experience feel native to the wider screen without any logic changes — pure CSS media query.

**Bundle size.** Dioxus WASM bundles for simple apps are in the 50–200 KB range after compression. Language pack JSON is loaded lazily on language select rather than bundled into the initial WASM binary, keeping the initial load fast. Target: initial WASM + HTML under 300 KB gzipped.

**Offline on web.** A service worker caches the WASM bundle, all language pack JSON, and static assets on first load. Subsequent visits work fully offline. The service worker is generated by the `dx bundle` pipeline; no manual configuration required.

**Persistence on web.** Progress is written to Supabase in real time, identical to mobile (see section 9.3). The anonymous session JWT is stored in `localStorage` and rehydrated on every page load — the user's progress is fully restored even after closing the tab or clearing the page, as long as `localStorage` is intact. If `localStorage` is cleared, the anonymous session is lost and progress cannot be recovered unless the user had upgraded to a named account. This is the primary prompt for the account upgrade CTA on web.

**Hosting.** The static web bundle is deployed to Cloudflare Pages on every release tag. The same Cloudflare project serves the marketing site at the root and the app at `/app`. CDN edge caching handles the WASM binary; no origin server is needed for the app itself.

---

## 11. Monetization

### 10.1 v1 — free with a module gate

Modules 1–3 are fully free with no time limit. Module 4 onwards requires either a one-time purchase or a subscription. This lets users experience the full core mechanic and early curriculum before any paywall friction.

### 10.2 Pricing model (TBD, to be validated)

| Option | Price | Notes |
|---|---|---|
| One-time purchase | $9.99 | Full lifetime access |
| Annual subscription | $4.99/year | Preferred for engagement |
| Monthly subscription | $1.99/month | Lower commitment |

The one-time purchase is expected to perform well with the developer demographic (they evaluate tools on value, not monthly commitment). Both options should be presented at the paywall with the one-time option highlighted.

### 10.3 No ads

Ads degrade the focused, friction-free experience the product depends on. No ads in any version.

---

## 12. Success metrics

### 10.1 Acquisition

- Day 1 installs
- App Store conversion rate (impressions → install)
- Source breakdown (organic search, social, referral)

### 10.2 Engagement

- D1 / D7 / D30 retention
- Average challenges completed per session
- Average session length
- Streak distribution (% of users with 3+ day streak at D7)

### 10.3 Learning quality

- First-attempt correct rate per challenge (identifies bad challenges)
- Hint usage rate per challenge (identifies hard challenges)
- Module completion rate

### 10.4 Revenue

- Conversion rate from free to paid at paywall
- ARPU
- Monthly recurring revenue

### 10.5 Target benchmarks (v1 launch)

| Metric | Target |
|---|---|
| D7 retention | ≥ 40% |
| D30 retention | ≥ 20% |
| Avg challenges per session | ≥ 5 |
| Paywall conversion (among users who reach it) | ≥ 15% |
| Module 1 completion rate | ≥ 70% |

---

## 13. Launch plan

### 12.1 v1 scope

Modules 1–6 (first output through structs), full token assembly mechanic, streak system, XP, badges, local-only persistence, iOS, Android, and web. Web ships as a static WASM bundle on Cloudflare Pages. All three platforms launch simultaneously.

### 12.2 v1.5 scope

Modules 7–10 (enums through collections), named account upgrade flow (email / OAuth via Supabase Auth), Supabase Realtime-powered leaderboard (opt-in), share-a-challenge URL deeplinks, free compose mode, second language pack.

### 12.3 v2 scope

WASM-based on-device compilation for open-ended challenges, user-generated challenge packs, community challenges, spaced repetition review mode for previously completed challenges.

### 12.4 Distribution

- App Store (iOS), Google Play (Android), and `tapcode.dev` web — simultaneous launch
- Web is the primary acquisition channel: zero friction, no install required, shareable challenge links
- Initial marketing: Rust community on Reddit (r/rust, r/learnrust), X/Twitter build-in-public thread, Hacker News Show HN on launch day — all links point to the web version first
- Press outreach to developer newsletters (TLDR, Pointer, Bytes)
- Challenge share links as an organic growth loop: every solved challenge has a one-tap share button that generates a `tapcode.dev/rust/mX/cY` URL

---

## 14. Open questions

| # | Question | Owner | Target resolution |
|---|---|---|---|
| 1 | Should the WASM compiler be explored for v1 given Rust's compile speed on mobile? | Engineering | Before v1 dev start |
| 2 | What is the right paywall module — 3 or 5? | Product | A/B test at launch |
| 3 | Should free compose mode be a v1 feature or v1.5? | Product | 2 weeks before v1 scope lock |
| 4 | Do we need a social/competitive layer (leaderboard, friends) to hit D30 retention targets? | Product | User interviews |
| 5 | How does the paywall work on web — can't use In-App Purchase APIs. Options: Stripe checkout redirecting back to the app, or web users get a free tier with a prompt to install the mobile app for full access. | Product / Engineering | Before v1 dev start |
| 11 | At what point do we prompt the anonymous user to upgrade to a named account? On web, `localStorage` loss is the forcing function. On mobile the pressure is lower since Keychain survives app reinstall. The CTA timing and copy needs a decision. | Product / Design | Before v1 dev start |
| 6 | Should hints cost XP? Early testing may show this creates anxiety for new users. | Design | First usability test |
| 7 | Which language ships second — Go, Python, or TypeScript? Go has the simplest syntax (fastest to author); Python has the largest learner audience; TypeScript has the highest overlap with the Rust persona. | Product | After v1 retention data |
| 8 | Should each language have its own paywall or is it one subscription for all languages? | Product | Before v1.5 planning |
| 9 | Community-authored language packs in v2 — what is the review and trust model? Who validates that a community compiler adapter is safe to run in the sandbox? | Engineering | v2 planning |
| 10 | Does the streak system count across all active languages or per-language? A user studying both Rust and Go should likely have one streak, not two. | Product | Before v1.5 dev start |

---

*End of document. Next step: resolve open questions 1 and 5 (WASM compiler viability + web paywall model), then move to design spike on responsive layout breakpoint, contextual chip highlighting, and challenge share URL format.*
