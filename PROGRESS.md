# Corvus Noctis progress

## 1. Branding

- Added the supplied README header, Corvus icons for the web and Mac app, and a 1280 × 640 social preview. The SVG wraps the approved raster artwork so it preserves the original mark.
- Verification: checked asset dimensions and output sizes, and inspected the Mac ZIP for its bundled icon and plist entry. CI builds the kernel and web edition and runs the pointer proof.
- Gabe: upload `docs/brand/social-preview.png` under **Settings → General → Social preview**.
- Gabe: set the repository description to “An operating system you can talk to. Rust, from scratch, TempleOS spirit.” and the website to `https://gabe-maxxing.github.io/corvus-noctis/`.
- Gabe: set the topics to `rust`, `operating-system`, `osdev`, `templeos`, `arm64`, `bare-metal`, `wasm`, `qemu`, and `corvus`.

## Kernel sprint / 1. Diogenes

- Pulled main (`6d8826f`) first. Ownership stays in src/, tools/, mac/, windows/ and the explicitly requested dist/install.sh; web/www and .github are untouched.
- Added `dio <question>`, `?question`, `dio` conversation mode and `exit`, with 45-second kernel timeout, Ctrl-C and bounded replies. Serial RS/US protocol bypasses framebuffer output. A cancelled/late framed reply never becomes shell input.
- The standard-library bridge calls only local Ollama `/api/chat`, defaults to llama3.2:3b, reads the existing persona, strips terminal control bytes, bounds output and suppresses cancelled generations. Missing Ollama gets an in-character install line. No model downloads or third-party Python packages are required.
- Integrated the bridge in touch.py, interactive qemu.sh, the packaged Mac app and the Unix launcher. Windows launches Python and fetches the two bridge files beside existing ZIPs, because release packaging belongs to Claude's workflow; bundling those files later removes that first-launch fetch. No .github edits.
- Web's shared shell compiles unchanged; Claude's JS currently intercepts `dio`, so the kernel protocol does not replace his browser model adapter. Exposed `tick(now_ms)` is available for that adapter's event loop.
- Validation: native shell contract and four bridge tests pass; kernel/web release builds have no warnings; Mac archive includes bridge/persona and both shell scripts pass syntax checks. The existing pointer proof now also exercises serial Diogenes and host contracts in CI. Local Homebrew QEMU installation was blocked by an unwritable existing /usr/local/libexec; do not change system directory ownership just for this task. CI's QEMU proof remains mandatory before merge.

## Kernel sprint / 2. Shared friendly shell

- Unknown commands now suggest the nearest command by bounded edit distance, then offer Diogenes; two consecutive misses add the plain-language invitation. A successful command resets the streak. This is in shell.rs, so kernel and web use identical text/lookup.
- Added 16-entry RAM history, arrow recall with draft restoration, and unique-prefix Tab completion (ambiguous prefixes list options). No heap or persistent transcript. Full command vocabulary stays in one table for future additions.
- Host contracts cover typo suggestions/streak reset, recall, draft preservation, completion and history eviction. Each CI pointer proof runs these alongside kernel protocol checks; the shared web build remains mandatory.
- PR #2 merged after kernel warnings check, prove_pointer PASS and web MVP/ES2017 build succeeded. CI still emits GitHub Action Node-version deprecation notices from Claude-owned .github; compiler warnings are zero. No workflow edits were made here.

## Kernel sprint / 3. Commands and bounded waiting

- Added generic monotonic time/uptime, honest mem accounting (reserved/static bytes, not invented free RAM), the 16-colour Corvus palette, large banner text, repeat, sleep, logo replay and original lantern text art. History/completion advertise the new commands.
- Sleep is an event-loop deadline, not a busy wait: pointer input and Ctrl-C stay responsive. The shared wasm shell schedules its own timer using existing wasm-bindgen, without editing web files. Repeat is capped at 32 and excludes nested/interactive/asynchronous commands to prevent runaway output and ambiguous queued prompts.
- Large text and logo effects are a narrow rendering boundary reusable by CrowC; the language stays in shell.rs. ANSI numeric parsing saturates instead of overflowing.
- Tests cover timer boundaries/cancellation, repeat limits, colour validation, memory wording and art commands. Kernel/web builds are warning-free; CI still gates the pointer proof and iOS-compatible web build. PR #3 merged after those checks passed.

## Kernel sprint / 4. CrowC M1-M3

- Added an original no-heap immediate interpreter: integer expressions/comparisons/short-circuit logic, quoted output, optional-parenthesis builtins, integer/string variables, Draw, if/else and while. Valid CrowC is tried before legacy commands; plain unknown words still receive friendly suggestions.
- Fixed bounds: 64 variables, 24-byte names, 128-byte strings, 1024-byte continued blocks, depth 32, 4096 steps/output bytes. Draw work also consumes the step budget. Arithmetic wraps deliberately; division by zero is an error. Reset clears the session arena. No JIT or executable memory.
- Multiline braces use `.. `; Ctrl-C clears continuation or interrupts evaluation. Serial type-ahead is deferred while polling cancellation. Shell state moved to static BSS so its arenas do not consume the 64 KiB kernel stack.
- Tests exercise strings/variables, precedence, short circuit, skipped branches, drawing/default colour, while limits, reset/exhaustion, continuation/cancellation, deep nesting and 512 deterministic malformed inputs. Added a direct cancellation-before-execution test. Rendering clips Draw to the existing framebuffer; no web source changes.
- CI caught a web regression before merging #4: Rust Closure callbacks introduced non-MVP reference types. Replaced them with a primitive-only timer import and an ES2017 module callback. The existing MVP and syntax checks now pass locally; no old-Safari check was weakened.

## Kernel sprint / 5. Noctis kernel chrome

- Added an original pixel crow/lantern, a timed 1.5-second animated boot splash, CORVUS NOCTIS title/tagline, palette swatches and HELP/DIO/CROW/CLEAR controls. `logo` replays the built-in mark even without an optional frames asset.
- Status line shows generic uptime and Dio readiness; the host bridge probes local installed models after boot and every 30 seconds. Status frames do not consume a pending answer. Buttons leave conversation/continuation safely before dispatching.
- Kernel layout reserves the status line below terminal rows. Target-specific chrome keeps Claude's web canvas layout/HELP hit target intact; shared commands and colours still compile there. No web/www or .github source changes.
- QEMU proof now checks all four controls and the actual ready-status pixels, and records PNG visual evidence in its existing CI log. This avoids changing Claude's artifact workflow. Local host contracts and warning-free kernel/web builds remain required; CI gates the kernel, pointer proof and MVP/ES2017 web output.
- PR #5 merged with real-kernel CrowC and pointer proof passing. The temporary browser timer regression from #4 was corrected before merge, and actual browser sleep/cancellation/import checks passed.
