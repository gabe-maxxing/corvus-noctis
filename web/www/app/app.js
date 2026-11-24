// Corvus web console. Plain ES2017 (no top-level await, no ?. or ??) so it
// runs on iOS 12 Safari; touch and mouse events because iOS 12 has no
// pointer events.
import init, * as corvus from "./pkg/corvus_web.js";
import * as dioModule from "./dio.js";

var canvas = document.getElementById("screen");
var ctx = canvas.getContext("2d");
var input = document.getElementById("input");
var form = document.getElementById("line");
var status = document.getElementById("status");

var wasm = null;
var W = 800, H = 600;
var line = ""; // mirrors the shell's line buffer, so web-only commands are caught on Enter
var busy = false;
var listeners = []; // other modules (tour, chips) hear about every submitted line

export var COLS = 76;
export var HELP = { x: 572, y: 16, w: 212, h: 120 };

export function draw() {
  var bytes = new Uint8ClampedArray(wasm.memory.buffer, corvus.pixels(), W * H * 4);
  ctx.putImageData(new ImageData(bytes, W, H), 0, 0);
}

export function onCommand(fn) { listeners.push(fn); }
export function isBusy() { return busy; }
export function setStatus(text) { status.textContent = text; }

// Same bytes a serial terminal would send: printable ASCII, CR, backspace.
function type(text) {
  if (busy) return;
  for (var i = 0; i < text.length; i++) {
    var code = text.charCodeAt(i);
    if (code >= 32 && code < 127) {
      corvus.key(code);
      if (line.length < 128) line += text[i];
    }
  }
}

function backspace() {
  if (busy) return;
  corvus.key(8);
  line = line.slice(0, -1);
}

var webCommands = {}; // name -> function(args): commands that live in JS, not in the kernel shell
export function webCommand(name, fn) { webCommands[name] = fn; }

// Kernel shell commands (src/shell.rs) and the web-only ones, for suggestions.
export var KNOWN = ["help", "about", "crow", "clear", "echo", "calc", "dio", "tour", "exit"];
var dioMode = false; // like the kernel's llm_mode: every line goes to Diogenes until "exit"
export function inDioMode() { return dioMode; }

function enter() {
  if (busy) return;
  var typed = line;
  var trimmed = typed.trim();
  var name = trimmed.split(/\s+/)[0];
  line = "";
  var handler = null, args = "";
  if (dioMode && trimmed === "exit") { handler = function () { dioMode = false; setPrompt(); }; }
  else if (trimmed.charAt(0) === "?") { handler = webCommands.dio; args = trimmed.slice(1).trim() || "?"; }
  else if (dioMode && trimmed) { handler = webCommands.dio; args = trimmed; }
  else if (webCommands.hasOwnProperty(name)) { handler = webCommands[name]; args = trimmed.slice(name.length).trim(); }
  if (handler) {
    for (var i = 0; i < typed.length; i++) corvus.key(8); // take it back out of the shell
    corvus.write(trimmed + "\n");
    fresh = true;
    Promise.resolve(handler(args)).then(done, function (error) {
      out("error: " + (error && error.message ? error.message : error));
      done();
    });
  } else if (dioMode) {
    for (var j = 0; j < typed.length; j++) corvus.key(8);
    corvus.write("\n");
    fresh = true;
    done();
  } else {
    corvus.key(13);
  }
  var unknown = !handler && !dioMode && name !== "" && KNOWN.indexOf(name) < 0;
  listeners.forEach(function (fn) { fn(trimmed, name, unknown); });
}

/// Types a whole command and presses Enter, like a person would.
export function run(command) {
  if (busy) return;
  while (line.length) backspace();
  type(command);
  enter();
  draw();
}

/// Puts text on the input line without running it.
export function prefill(text) {
  if (busy) return;
  while (line.length) backspace();
  type(text);
  draw();
}

// Console writer for JS output: ASCII only, word-wrapped to the 76 columns.
var col = 0, word = "", fresh = false; // fresh: cursor at the start of a line after an echo
function ascii(text) {
  return text.normalize("NFKD").replace(/[̀-ͯ]/g, "")
    .replace(/[‘’]/g, "'").replace(/[“”]/g, '"').replace(/[–—]/g, "-")
    .replace(/…/g, "...").replace(/[^\x20-\x7e\n]/g, "");
}
function flushWord() {
  if (!word) return;
  if (col + word.length > COLS && col > 0) { corvus.write("\n"); col = 0; }
  corvus.write(word);
  col += word.length;
  word = "";
}
export function out(text) {
  var clean = ascii(text);
  if (clean) fresh = false;
  for (var i = 0; i < clean.length; i++) {
    var ch = clean[i];
    if (ch === "\n") { flushWord(); corvus.write("\n"); col = 0; }
    else if (ch === " ") { flushWord(); if (col > 0 && col < COLS) { corvus.write(" "); col++; } }
    else word += ch;
  }
  draw();
}
/// Raw console write (no wrapping), e.g. for "\r" progress bars.
export function raw(text) { corvus.write(text); draw(); }
var promptText = "corvus> ";
function setPrompt() { promptText = dioMode ? "diogenes> " : "corvus> "; }
function done() { flushWord(); corvus.write((fresh ? "" : "\n") + promptText); col = 0; fresh = false; draw(); }
/// Enters Diogenes mode as if "dio" was typed (used by the tour for non-technical people).
export function openDioMode() {
  if (busy || dioMode) return;
  run("dio");
}
export function setBusy(value) { busy = value; }

// --- dio: ask Diogenes ---------------------------------------------------
webCommand("dio", function (question) {
  if (!question) {
    dioMode = true;
    setPrompt();
    out("Talk plainly. Type exit to return.");
    return;
  }
  busy = true;
  setStatus("diogenes is lighting his lantern...");
  var lastBar = -1;
  function progress(fraction) {
    var pct = Math.floor(fraction * 100);
    setStatus("loading diogenes " + pct + "% (first time only)");
    if (pct === lastBar) return;
    lastBar = pct;
    var filled = Math.round(fraction * 30);
    raw("\r[" + "#".repeat(filled) + ".".repeat(30 - filled) + "] " + String(pct).padStart(3) + "% loading lantern");
  }
  function clearBar() { if (lastBar >= 0) raw("\r" + " ".repeat(COLS - 1) + "\r"); }
  var started = false;
  return Promise.resolve(dioModule).then(function (m) {
    return m.ask(question, function (text) {
      if (!started) { started = true; clearBar(); out("dio> "); }
      out(text);
    }, progress);
  }).then(function (backend) {
    setStatus("running · dio via " + backend);
    busy = false;
  }, function (error) {
    clearBar();
    out("dio: " + (error && error.message ? error.message : error) +
      "\nneeds WebGPU (Safari 26+, Chrome) or a local Ollama: OLLAMA_ORIGINS=* ollama serve");
    setStatus("running");
    busy = false;
  });
});

// --- input ---------------------------------------------------------------
input.addEventListener("input", function () {
  type(input.value);
  input.value = "";
  draw();
});
input.addEventListener("keydown", function (event) {
  if (event.key === "Backspace" || event.keyCode === 8) { backspace(); draw(); event.preventDefault(); }
});
form.addEventListener("submit", function (event) {
  event.preventDefault();
  type(input.value);
  input.value = "";
  enter();
  draw();
});

function toFramebuffer(clientX, clientY) {
  var r = canvas.getBoundingClientRect();
  return [Math.floor((clientX - r.left) * W / r.width), Math.floor((clientY - r.top) * H / r.height)];
}
function hitsHelp(p) { return p[0] >= HELP.x && p[0] < HELP.x + HELP.w && p[1] >= HELP.y && p[1] < HELP.y + HELP.h; }
var down = false;
function press(clientX, clientY) {
  if (busy) return;
  var p = toFramebuffer(clientX, clientY);
  if (hitsHelp(p)) line = ""; // HELP clears the line, like the kernel
  down = true;
  corvus.pointer(p[0], p[1], true);
  draw();
  if (!hitsHelp(p)) input.focus();
}
function release(clientX, clientY) {
  if (!down) return;
  down = false;
  var p = toFramebuffer(clientX, clientY);
  corvus.pointer(p[0], p[1], false);
  draw();
}
if (window.PointerEvent) {
  canvas.addEventListener("pointerdown", function (e) { press(e.clientX, e.clientY); });
  ["pointerup", "pointercancel", "pointerleave"].forEach(function (name) {
    canvas.addEventListener(name, function (e) { release(e.clientX, e.clientY); });
  });
} else {
  canvas.addEventListener("touchstart", function (e) {
    var t = e.changedTouches[0];
    e.preventDefault();
    press(t.clientX, t.clientY);
  });
  canvas.addEventListener("touchend", function (e) {
    var t = e.changedTouches[0];
    release(t.clientX, t.clientY);
  });
  canvas.addEventListener("mousedown", function (e) { press(e.clientX, e.clientY); });
  canvas.addEventListener("mouseup", function (e) { release(e.clientX, e.clientY); });
}

// --- boot ----------------------------------------------------------------
var splash = document.getElementById("splash");
var splashBar = document.getElementById("splash-bar");
var splashLine = document.getElementById("splash-line");
var bootStart = Date.now();
var CAWS = ["lighting the lantern...", "polishing 16 colours...", "waking the crow...", "looking for a man..."];
var cawIndex = 0;
var cawTimer = setInterval(function () { cawIndex = (cawIndex + 1) % CAWS.length; if (splashLine) splashLine.textContent = CAWS[cawIndex]; }, 700);
function splashProgress(pct) { if (splashBar) splashBar.style.width = pct + "%"; }
function hideSplash() {
  clearInterval(cawTimer);
  splashProgress(100);
  setTimeout(function () {
    if (!splash) return;
    splash.className = "gone";
    setTimeout(function () { splash.parentNode && splash.parentNode.removeChild(splash); }, 500);
  }, Math.max(0, 700 - (Date.now() - bootStart)));
}
splashProgress(35);

export var ready = init({ module_or_path: "pkg/corvus_web_bg.wasm" }).then(function (exports) {
  splashProgress(80);
  wasm = exports;
  W = corvus.width();
  H = corvus.height();
  corvus.boot();
  draw();
  (function frame(now) {
    if (corvus.tick(now)) draw();
    requestAnimationFrame(frame);
  })(0);
  setStatus("running");
  hideSplash();
  if (window.matchMedia && matchMedia("(pointer: fine)").matches) input.focus();
}, function (error) {
  clearInterval(cawTimer);
  if (splashLine) splashLine.textContent = "the crow fell off its perch: " + (error && error.message ? error.message : error);
  throw error;
});

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("../sw.js", { scope: "../" }).catch(function () { /* offline cache is a bonus */ });
}
