# Clipboard History for macOS

A small clipboard history utility written in Rust for macOS.

This project exists for a very simple reason: on Windows, I can use **Win + V** for clipboard history, and on Fedora I can use a GNOME extension for the same purpose. On macOS, I did not have a built-in solution that felt equivalent for my use case, so I decided to build one.

The application runs in the background, watches the clipboard, stores recent text entries, and lets me restore previous entries through a tray-based UI.

---

## Why this project exists

This is a personal utility built to solve **my clipboard history problem on a Mac mini**.

That context matters.

If this were meant to be a battery-sensitive laptop utility for general distribution, I would be much more concerned about minimizing background activity. But this app was designed primarily for a **desktop Mac mini**, where the cost of a lightweight polling loop is acceptable for my use case.

The goal of the project was not to create the most sophisticated clipboard manager possible. The goal was to build something:

- practical
- understandable
- small enough to maintain
- structured enough to not turn into spaghetti

---

## How it works

The project is split into a few backend components:

- **ClipboardManager**  
  Responsible for reading from and writing to the system clipboard.

- **ContentManager**  
  Periodically checks the clipboard, compares the current content against the last observed value, and forwards new content to the history layer.

- **HistoryManager**  
  Stores clipboard history in memory with a maximum capacity.

The UI is exposed through a tray icon, allowing the window to stay hidden until needed.

---

## Architecture notes

Even though this is a small project, I wanted the backend to have clear separation of responsibilities.

Instead of mixing clipboard access, storage, and UI concerns together, the program uses separate managers and message passing between them. This makes the behavior easier to reason about and keeps the clipboard/history logic independent from the interface layer.

The current history storage uses a `VecDeque<String>`.

That was a deliberate choice. The usage pattern is:

- push the newest item to the front
- remove the oldest item from the back once the history reaches its maximum size
- occasionally read the collection to display it in the UI

A plain `Vec` would have worked, but `VecDeque` is a better fit for this insertion/removal pattern.

---

## Trade-offs

### 1. Polling instead of clipboard event subscription

The biggest design trade-off in this project is the clipboard watcher.

On macOS, I did not find a simple API that let me subscribe to clipboard changes in the way I originally wanted. Because of that, the program uses a polling loop:

1. read current clipboard text
2. compare it to the last seen value
3. store it if it changed
4. sleep briefly
5. repeat

This is not the most elegant approach, but it is the practical one under the platform constraints I was working with.

I considered this acceptable because:

- the target machine is a **Mac mini**, not a battery-powered laptop
- the work being done each cycle is lightweight
- the app solves a real daily annoyance for me
- the implementation stays simple enough to maintain

If I were optimizing for laptop battery life or building a broader consumer app, I would be much more critical of this decision.

### 2. Bounded in-memory history

The history is intentionally capped at a fixed size instead of growing forever.

That keeps the project simple and predictable. For my use case, I do not need persistent long-term clipboard storage; I only need quick access to recent entries.

### 3. Personal tool first, generalized product second

This project was built as a tool for my own workflow, not as a polished product meant to cover every edge case.

That shaped a lot of the decisions:
- in-memory storage instead of persistence
- polling instead of a more complex system-level integration
- a simple tray UI instead of a richer interface
- prioritizing clarity over feature breadth

---

## About the UI and AI assistance

I want to be transparent about how this project was built.

I wrote the backend logic myself, including the clipboard/history/content management parts and the architectural decisions around them.

However, I do **not** have much experience with frontend/UI libraries, especially in Rust desktop development. Because of that, I used a mix of **ChatGPT, Codex, and Gemini** to help generate the UI-related code.

More specifically:

- **100% of `main.rs` was AI-assisted/generated**
- the tray/window/UI integration was not written entirely by me from scratch
- I reviewed, tested, adjusted, and integrated that code into the project

I am explicitly stating this because I think it is better to be honest than to pretend otherwise.

For me, the value of this project is still real:
- I understood the problem I wanted to solve
- I designed the backend structure
- I made the core trade-offs
- I validated the behavior on my machine
- I stitched the pieces together into a working utility

The UI layer reflects a part of Rust I am still learning.

---

## Current limitations

This is still a small personal utility, and it has limitations:

- text clipboard history only
- no persistent storage across restarts
- polling-based clipboard detection
- not designed with laptop battery optimization as the top priority
- UI code is functional, but not the part of the project I would consider the strongest technically

---

## Why I still like this project

I like this project because it sits in a good middle ground.

It is not so simple that it teaches nothing, and it is not so complicated that it collapses under its own weight.

It solves a real annoyance in my daily setup, and it gave me a chance to think about:

- responsibility separation
- message passing between components
- data structure selection
- platform constraints
- practical engineering trade-offs

That is exactly the kind of project I enjoy building.

---

## Future improvements

Some things I may improve later:

- make polling interval configurable
- improve error reporting/logging
- support persistent history storage
- refine the UI further
- handle richer clipboard content types beyond plain text

---

## Running the project

I personally run this project by building it in release mode and launching the executable from `target/release` whenever I start my Mac.

If you want it to start automatically, you can add the executable to macOS login items so it runs at startup.

Build the project with:

```bash
cargo build --release
```

## Inspiration

Part of the motivation for this project came from using **Clipboard History by SUPERCILEX** on GNOME, which solved this problem really well for me on Fedora.

That experience made me want something similar on macOS, where I did not have an equivalent built-in workflow for clipboard history.

GNOME extension:
[Clipboard History by SUPERCILEX](https://extensions.gnome.org/extension/4839/clipboard-history/)