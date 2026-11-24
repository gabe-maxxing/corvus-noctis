# Onboarding

How a new person gets from the link to talking to Corvus. Files: `web/www/index.html` +
`landing.js` (download page), `web/www/app/tour.js` + `app.js` (first start in the app),
`web/www/sw.template.js` (offline).

## 1. The download page asks three questions

Big buttons, one question per screen, progress bar, back button. The crow guide talks in a
speech bubble: short, a bit funny, never more than two lines.

| # | Question | Answers | Default |
|---|----------|---------|---------|
| 1 | Where will Corvus live? | iPad/iPhone, Android, Mac, Windows, Linux/Unix | detected device is highlighted ("tap it, or lie to me") |
| 2 | What do you want to do? | Code, Just play, Talk to Diogenes | none |
| 3 | Are you technical? | Yes ("give me the commands"), No ("hold my hand") | none |

The answers become a **profile**: `<device>-<goal>-<tech>`, e.g. `ios-talk-0`, `windows-code-1`.

- Saved in `localStorage["corvus.profile"]` (same origin as the app, so the app sees it too).
- Put in the page URL (`?profile=ios-talk-0`), the web-app link (`app/?profile=...`) and the
  download links (`.../Corvus-mac.zip?profile=...`; GitHub ignores the query, it is there so
  the link says who it was for).
- A returning visitor skips the questions and lands on their guide ("caw, welcome back").
  "Change my answers" starts over.

## 2. The guide, one step at a time

After question 3 there is one big button (open the web app, download the zip, or copy the
install command) and a guide per device. One step per screen, `step n of N` with a progress
bar, back/next. Each step has a title, one line of what to do, the crow's line, and a picture:
a **real screenshot** of Corvus (boot splash, console) or a **labelled illustration** of the
OS dialog (Safari Share sheet, Gatekeeper, SmartScreen, Terminal). Never "drag the DMG and
good luck".

| Device | Steps |
|--------|-------|
| iPad / iPhone | open web app → tap Share → Add to Home Screen → open from home screen (offline) |
| Android | open web app → Chrome ⋮ menu → Add to Home screen / Install app → open |
| Mac | download zip → move to Applications → first open (Privacy & Security → Open Anyway; older macOS right-click → Open) → let it `brew install qemu` → Corvus boots |
| Windows | download zip → Extract All → run Corvus.bat (SmartScreen: More info → Run anyway) → yes to `winget` QEMU → Corvus boots |
| Linux / Unix | copy `curl -fsSL …/install.sh \| sh` → paste in a terminal (sudo is only for QEMU) → `corvus` |

Technical = yes: shorter crow lines, raw commands. Technical = no: more hand-holding (what a
zip is, why macOS warns, what QEMU is). Desktop guides end with **Run in VM (recommended)**
and an honest **Dual boot (experimental, coming)**: not possible yet, Corvus only has QEMU
drivers.

## 3. First start of the app: the welcome tour

`app/tour.js` reads the profile from `?profile=` (and saves it) or from localStorage. If
`localStorage["corvus.tour"] !== "done"`, a 5-step tour opens at the bottom (the console stays
usable). Skip ends it; finishing or skipping sets `corvus.tour = "done"`. Type `tour` to
replay it.

| Step | Everyone | Coder (`code`) | Player (`play`) | Talker / not technical |
|------|----------|----------------|-----------------|------------------------|
| 1 | welcome: a real OS shell in the browser, offline too | | | **Diogenes mode opens right away** (`diogenes>`) |
| 2 | HELP button = `help`; input bar; chips are shortcuts | | | |
| 3 | | CrowC taste: `"caw";`, `Draw(...)` (evaluator with Codex); `calc (2 + 3) * 4` typed for you | `crow` typed for you | "type like you'd text a grumpy friend; `exit` leaves" |
| 4 | | the shell speaks `calc` until CrowC lands; where the source is | 16 colours, one crow, one lantern; try `about` | talk: model downloads once / Ollama; not technical + play: `exit`, then `crow` |
| 5 | "type `tour` to see this again. caw." | | | |

Rule: not technical + goal is not `code` → Diogenes mode is open at step 1.

## 4. Help in the console

- Unknown command: the kernel shell (Codex, `src/shell.rs`) answers "`x` isn't a command. Did
  you mean `y`? Or just ask Diogenes: dio <your question>". The web console adds chips:
  **ask dio: x** (highlighted, one tap), **y**, **dio**, **help**.
- `dio` alone = Diogenes mode until `exit`; `dio <q>` and `?<q>` ask once. Same as the kernel;
  in the web console these are handled in JS (WebLLM, else local Ollama).
- Up/Down = history, Tab = complete (several matches become chips).

## 5. Offline and old iPads

- Service worker, cache-first, one cache per commit (`sw.js` is stamped with the sha by
  `.github/scripts/build-web.sh`); precaches pages, app JS, wasm, icons, logo, guide images.
  Loaded once → works offline.
- iOS 12 Safari (iPad Air 1): MVP WebAssembly only (wasm-opt lowers sign-ext/bulk-memory, the
  build fails on anything else), ES2017 JavaScript (acorn check), touch events when there are
  no pointer events, no CSS that needs Safari 13+. Diogenes on iOS 12: no WebGPU, so `dio` says
  so and points to Ollama.

## Test it

```sh
.github/scripts/build-web.sh dev && python3 -m http.server -d web/www 8000
# fresh profile:   localStorage.clear() in the console, reload /
# straight to app: /app/?profile=ios-talk-0   /app/?profile=windows-code-1
```
