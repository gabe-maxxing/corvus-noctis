// Welcome tour (5 steps, tailored to the profile from the download page),
// the suggestion chips under the console, and `tour` to replay it.
// Plain ES2017 for iOS 12 Safari. Flow: docs/onboarding.md.
import { run, prefill, onCommand, openDioMode, webCommand, ready, out, inDioMode } from "./app.js";

var PROFILE_KEY = "corvus.profile";
var TOUR_KEY = "corvus.tour";

function parse(s) {
  var m = /^(ios|android|mac|windows|linux)-(code|play|talk)-([01])$/.exec(s || "");
  return m ? { device: m[1], goal: m[2], tech: m[3] === "1" ? "yes" : "no" } : null;
}
function profile() {
  var p = parse(new URLSearchParams(location.search).get("profile"));
  if (p) { try { localStorage.setItem(PROFILE_KEY, JSON.stringify(p)); } catch (e) { /* private mode */ } return p; }
  try { return JSON.parse(localStorage.getItem(PROFILE_KEY)) || null; } catch (e) { return null; }
}
function seen() { try { return localStorage.getItem(TOUR_KEY) === "done"; } catch (e) { return false; } }
function markSeen() { try { localStorage.setItem(TOUR_KEY, "done"); } catch (e) { /* ignore */ } }

// ---- chips ----------------------------------------------------------------
var chips = document.getElementById("chips");
function setChips(list, highlight) {
  chips.innerHTML = list.map(function (c, i) {
    return '<button type="button" class="chip' + (highlight && i === 0 ? " hot" : "") + '" data-run="' +
      c.run.replace(/"/g, "&quot;") + '"' + (c.prefill ? ' data-prefill="1"' : "") + ">" + c.label + "</button>";
  }).join("");
}
function defaultChips() {
  if (inDioMode()) {
    setChips([{ label: "what is virtue?", run: "what is virtue?" }, { label: "roast my phone", run: "roast my phone" }, { label: "exit", run: "exit" }]);
  } else {
    setChips([{ label: "help", run: "help" }, { label: "calc 6*7", run: "calc 6*7" }, { label: "dio …", run: "dio " , prefill: true },
      { label: "crow", run: "crow" }, { label: "tour", run: "tour" }]);
  }
}
chips.addEventListener("click", function (e) {
  var b = e.target.closest("button.chip");
  if (!b) return;
  if (b.getAttribute("data-prefill")) prefill(b.getAttribute("data-run"));
  else run(b.getAttribute("data-run"));
  document.getElementById("input").focus();
});
onCommand(function (line, name, unknown) {
  if (unknown) {
    // The kernel answers "Unknown command"; offer Diogenes, one tap away.
    setChips([{ label: "ask dio: " + (line.length > 24 ? line.slice(0, 24) + "…" : line), run: "dio " + line },
      { label: "help", run: "help" }, { label: "tour", run: "tour" }], true);
  } else {
    setTimeout(defaultChips, 0);
  }
});

// ---- tour -----------------------------------------------------------------
var box = document.getElementById("tour");
var text = document.getElementById("tour-text");
var bar = document.getElementById("tour-bar");
var label = document.getElementById("tour-label");
var nextBtn = document.getElementById("tour-next");
var steps = [], index = 0;

function stepsFor(p) {
  var goal = p ? p.goal : "play";
  var tech = p ? p.tech === "yes" : false;
  var first = { say: "caw. welcome to Corvus. this is a real OS shell, compiled to run in your browser. offline, too." };
  var screen = { say: "the big purple HELP button works like typing <code>help</code>. the bar below is your keyboard. the buttons under it are shortcuts." };
  var last = { say: "that's the tour. type <code>tour</code> to see it again. now go poke things. caw." };
  var middle = {
    talk: [
      { say: "Diogenes is open. just type a question, like <code>what is virtue?</code> he's blunt. it's a lifestyle.", enter: function () { openDioMode(); } },
      { say: "the first answer downloads a small model, once. no WebGPU? run Ollama on your computer. type <code>exit</code> to leave." },
    ],
    code: [
      { say: "CrowC is coming: HolyC-style, everything live. a taste: <code>\"caw\";</code> prints caw, <code>Draw(...)</code> paints. Codex is building the evaluator.",
        enter: function () { prefill("calc (2 + 3) * 4"); } },
      { say: "until then the shell speaks <code>calc</code>. press Enter on the line I typed for you. source: <a href=\"https://github.com/gabe-maxxing/corvus-noctis\" target=\"_blank\" rel=\"noopener\">github</a>, src/shell.rs." },
    ],
    play: [
      { say: "type <code>crow</code>. that's me. say hi.", enter: function () { prefill("crow"); } },
      { say: "16 colours, one crow, one lantern. that's the whole palette and we're proud of it. tap HELP, try <code>about</code>." },
    ],
  }[goal];
  if (!tech && goal !== "code") {
    // Non-technical: Diogenes mode is open right away, no commands to remember.
    first.enter = function () { openDioMode(); };
    middle = [
      { say: "Diogenes is already listening: the prompt says <code>diogenes&gt;</code>. type like you'd text a grumpy friend. <code>exit</code> leaves." },
      goal === "talk"
        ? { say: "his first answer downloads a small model, once. no WebGPU on this device? he'll tell you, and Ollama on a computer works too." }
        : { say: "16 colours, one crow, one lantern. type <code>exit</code>, then <code>crow</code> to meet me properly." },
    ];
  }
  var list = [first, screen].concat(middle).concat([last]);
  return list;
}

function show(i) {
  index = i;
  var s = steps[i];
  text.innerHTML = s.say;
  label.textContent = (i + 1) + " / " + steps.length;
  bar.style.width = ((i + 1) / steps.length * 100) + "%";
  nextBtn.textContent = i === steps.length - 1 ? "let's go" : "next";
  if (s.enter) s.enter();
  defaultChips();
}
export function startTour() {
  steps = stepsFor(profile());
  box.hidden = false;
  show(0);
}
function endTour() {
  box.hidden = true;
  markSeen();
  document.getElementById("input").focus();
}
nextBtn.addEventListener("click", function () { if (index < steps.length - 1) show(index + 1); else endTour(); });
document.getElementById("tour-skip").addEventListener("click", endTour);

webCommand("tour", function () {
  out("caw. starting the tour.");
  setTimeout(startTour, 0);
});

ready.then(function () {
  defaultChips();
  if (!seen()) startTour();
});
