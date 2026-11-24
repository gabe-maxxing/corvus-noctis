// Corvus download page: three questions, then a step-by-step guide with the
// crow as guide. Plain ES2017 for iOS 12 Safari. Flow: docs/onboarding.md.
(function () {
  "use strict";
  var REL = "https://github.com/gabe-maxxing/corvus-noctis/releases/download/nightly/";
  var INSTALL = new URL("install.sh", location.href).href;
  var KEY = "corvus.profile";
  var $ = function (id) { return document.getElementById(id); };

  // ---- profile -----------------------------------------------------------
  var DEVICES = {
    ios: { label: "iPad / iPhone", emoji: "📱", note: "web app, add to home screen" },
    android: { label: "Android", emoji: "🤖", note: "web app, add to home screen" },
    mac: { label: "Mac", emoji: "🍎", note: "Corvus.app in a VM" },
    windows: { label: "Windows", emoji: "🖥", note: "Corvus.bat in a VM" },
    linux: { label: "Linux / Unix", emoji: "🐧", note: "one curl command" },
  };
  var GOALS = {
    code: { label: "Code", emoji: "⌨️", note: "CrowC, the shell, the guts" },
    play: { label: "Just play", emoji: "🎨", note: "the crow, the logo, the colours" },
    talk: { label: "Talk to Diogenes", emoji: "🏮", note: "a cynic with a lantern" },
  };
  var TECH = {
    yes: { label: "Yes", emoji: "🛠️", note: "give me the commands" },
    no: { label: "No", emoji: "🙂", note: "keep it simple, hold my hand" },
  };

  function detect() {
    var ua = navigator.userAgent;
    if (/iPhone|iPad|iPod/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1)) return "ios";
    if (/Android/.test(ua)) return "android";
    if (/Macintosh|Mac OS X/.test(ua)) return "mac";
    if (/Windows/.test(ua)) return "windows";
    if (/Linux|X11|BSD|SunOS/.test(ua)) return "linux";
    return null;
  }

  function load() {
    var fromUrl = new URLSearchParams(location.search).get("profile");
    if (fromUrl) { var p = parse(fromUrl); if (p) return p; }
    try { return JSON.parse(localStorage.getItem(KEY)) || null; } catch (e) { return null; }
  }
  function save(p) { try { localStorage.setItem(KEY, JSON.stringify(p)); } catch (e) { /* private mode */ } }
  function encode(p) { return p.device + "-" + p.goal + "-" + (p.tech === "yes" ? 1 : 0); }
  function parse(s) {
    var m = /^(ios|android|mac|windows|linux)-(code|play|talk)-([01])$/.exec(s || "");
    return m ? { device: m[1], goal: m[2], tech: m[3] === "1" ? "yes" : "no" } : null;
  }

  // ---- the crow ----------------------------------------------------------
  function say(text) { $("say").innerHTML = text; }

  // ---- questions ---------------------------------------------------------
  var answers = {};
  var QUESTIONS = [
    { key: "device", title: "Where will Corvus live?", options: DEVICES,
      crow: function () { var d = detect(); return d ? "caw. looks like you're on <b>" + DEVICES[d].label + "</b>. tap it, or lie to me." : "caw. which machine are we haunting?"; } },
    { key: "goal", title: "What do you want to do?", options: GOALS,
      crow: function () { return "good. now: what are you here for? no wrong answers. some funny ones."; } },
    { key: "tech", title: "Are you technical?", options: TECH,
      crow: function () { return "last one. be honest. I've seen people paste things into terminals that they shouldn't."; } },
  ];
  var qIndex = 0;

  function ask(i) {
    qIndex = i;
    var q = QUESTIONS[i];
    $("ask").hidden = false;
    $("result").hidden = true;
    $("ask-label").textContent = "question " + (i + 1) + " of " + QUESTIONS.length;
    $("ask-bar").style.width = (i / QUESTIONS.length * 100) + "%";
    $("ask-title").textContent = q.title;
    $("back").style.visibility = i === 0 ? "hidden" : "visible";
    say(q.crow());
    var detected = q.key === "device" ? detect() : null;
    var html = "";
    Object.keys(q.options).forEach(function (k) {
      var o = q.options[k];
      var chosen = answers[q.key] === k || (!answers[q.key] && detected === k);
      html += '<button type="button" class="choice' + (chosen ? " detected" : "") + '" data-key="' + q.key + '" data-value="' + k + '">' +
        '<span class="emoji" aria-hidden="true">' + o.emoji + "</span>" + o.label + "<small>" + o.note + "</small></button>";
    });
    $("choices").innerHTML = html;
  }

  $("choices").addEventListener("click", function (e) {
    var b = e.target.closest("button.choice");
    if (!b) return;
    answers[b.getAttribute("data-key")] = b.getAttribute("data-value");
    if (qIndex + 1 < QUESTIONS.length) ask(qIndex + 1);
    else finish();
  });
  $("back").addEventListener("click", function () { if (qIndex > 0) ask(qIndex - 1); });
  $("restart").addEventListener("click", function () { answers = {}; ask(0); });

  // ---- mock screens (illustrations) ----------------------------------------
  function mock(title, body, dark) {
    return '<div class="shot"><div class="mock' + (dark ? " dark" : "") + '"><div class="title">' + title + '</div><div class="body">' + body +
      '</div></div><div class="cap">illustration, your screen may differ slightly</div></div>';
  }
  function real(src, alt) {
    return '<div class="shot"><img src="' + src + '" alt="' + alt + '"><div class="cap">real screenshot of Corvus</div></div>';
  }
  var SHOTS = {
    safariShare: mock("Safari", 'corvus-noctis …<div class="bar"><span>‹</span><span>›</span><span class="hl">⬆︎</span><span>📖</span><span>⧉</span></div>'),
    shareSheet: mock("Share", '<div>Copy</div><div>Add to Reading List</div><div class="hl">＋ Add to Home Screen</div><div>Find on Page</div>'),
    homeScreen: mock("Home Screen", '<div class="appgrid"><span><i></i>Notes</span><span><i></i>Photos</span><span><img src="icons/apple-touch-icon-120x120.png" alt="" class="hl">Corvus</span><span><i></i>Music</span></div>'),
    androidMenu: mock("Chrome", '<div style="text-align:right" class="hl">⋮</div><div>New tab</div><div>Bookmarks</div><div class="hl">Add to Home screen / Install app</div>'),
    macDownload: mock("Downloads", '<div class="hl">🗜 Corvus-mac.zip</div><div style="color:#777">double-click to unzip → Corvus.app</div>'),
    macDrag: mock("Finder", '<div><span class="hl">🐦 Corvus.app</span> &nbsp;→&nbsp; 📁 Applications</div>'),
    gatekeeper: mock("macOS", '<b>“Corvus” Not Opened</b><div>Apple could not verify “Corvus” is free of malware…</div><span class="btnm">Done</span><span class="btnm">Move to Trash</span>' +
      '<hr><div>System Settings → Privacy &amp; Security → <span class="hl">Open Anyway</span></div>'),
    brew: mock("Terminal", 'Corvus needs QEMU (a free virtual machine).<br>Install it now with Homebrew? [Y/n] <span class="hl">y</span>', true),
    winExtract: mock("File Explorer", '<div>🗜 Corvus-windows.zip</div><div>right-click → <span class="hl">Extract All…</span></div>'),
    smartscreen: mock("Windows protected your PC", '<div>Microsoft Defender SmartScreen prevented an unrecognized app from starting.</div><div class="hl" style="display:inline-block;margin-top:6px">More info</div> → <span class="btnm hl">Run anyway</span>'),
    winget: mock("Corvus OS", 'Corvus needs QEMU, a free virtual machine.<br>Install it now with winget? [Y/n] <span class="hl">y</span>', true),
    linuxTerm: mock("Terminal", '$ <span class="hl">curl -fsSL …/install.sh | sh</span><br>corvus installing QEMU…<br>corvus now run: corvus', true),
    splash: real("guide/splash.png", "The Corvus boot splash with the crow and its lantern"),
    console: real("guide/console.png", "The Corvus console after typing calc (2 + 3) * 4"),
  };

  // ---- guides ------------------------------------------------------------
  function copyable(cmd) {
    return '<code>' + cmd + '</code><button class="copy" type="button" data-copy="' + cmd.replace(/"/g, "&quot;") + '">copy</button>';
  }
  function goalLine(p) {
    return { code: "then type <code>tour</code> and I'll show you where the code lives.",
      play: "then tap HELP, type <code>crow</code>, and poke everything. nothing breaks. probably.",
      talk: "then type <code>dio</code> and a question. he's rude. it's his whole thing." }[p.goal];
  }
  function guide(p) {
    var t = p.tech === "yes";
    var app = "app/?profile=" + encode(p);
    var G = {
      ios: [
        { h: "Open the web app", crow: t ? "it's a PWA. you know the drill." : "tap the big yellow button. that's it. that's step one. you're doing great.",
          body: "It opens in Safari and runs right there, no App Store.", shot: SHOTS.splash },
        { h: "Tap Share", crow: "the square with the arrow. bottom of the screen on iPhone, top on iPad.", body: "Safari's Share button.", shot: SHOTS.safariShare },
        { h: "Add to Home Screen", crow: t ? "scroll, tap, done." : "scroll the list down a bit. it's hiding. like me in daylight.", body: "Tap <b>Add to Home Screen</b>, then <b>Add</b>.", shot: SHOTS.shareSheet },
        { h: "Open Corvus from your home screen", crow: "fullscreen, own icon, works offline. it's basically an app now. don't tell Apple.", body: goalLine(p), shot: SHOTS.homeScreen },
      ],
      android: [
        { h: "Open the web app", crow: "big yellow button. press it like you mean it.", body: "It runs in Chrome, no Play Store.", shot: SHOTS.splash },
        { h: "Open Chrome's menu", crow: "three dots, top right. the holy trinity of menus.", body: "Tap <b>⋮</b>.", shot: SHOTS.androidMenu },
        { h: "Add to Home screen", crow: "or 'Install app'. Chrome can't decide what to call it either.", body: "Tap <b>Add to Home screen</b> (or <b>Install app</b>).", shot: SHOTS.androidMenu },
        { h: "Open Corvus", crow: "there I am, on your home screen. caw.", body: goalLine(p), shot: SHOTS.console },
      ],
      mac: [
        { h: "Download Corvus-mac.zip", crow: t ? "unsigned .app in a zip, boots the kernel in QEMU." : "tap the big button. a zip lands in Downloads. zips are just boxes. we open it next.",
          body: "Safari usually unzips it for you into <b>Corvus.app</b>. If not, double-click the zip.", shot: SHOTS.macDownload },
        { h: "Move it to Applications", crow: "drag me into Applications. gently. I bite.", body: "Optional, but tidy. Corvus doesn't touch anything else on your Mac.", shot: SHOTS.macDrag },
        { h: "Open it the first time", crow: t ? "unsigned, so Gatekeeper complains. Open Anyway." : "macOS will say it can't check me. that's because I'm new, not because I'm evil. mostly.",
          body: "Double-click Corvus. If macOS blocks it: <b>System Settings → Privacy &amp; Security</b>, scroll down, <b>Open Anyway</b>. (Older macOS: right-click Corvus → Open → Open.)", shot: SHOTS.gatekeeper },
        { h: "Let it install QEMU", crow: t ? "brew install qemu, prompted. say y." : "a Terminal window asks to install QEMU, the free virtual machine I live in. type y and press return.",
          body: t ? "Needs Homebrew; it offers <code>brew install qemu</code>." : "No Homebrew yet? Terminal tells you to get it from <a href=\"https://brew.sh\">brew.sh</a> first: one copy-paste.", shot: SHOTS.brew },
        { h: "Corvus boots", crow: "a window with CORVUS in gold. type in the Terminal or click HELP. " + (p.goal === "talk" ? "Diogenes lives in the web app for now." : ""),
          body: goalLine(p), shot: SHOTS.console },
      ],
      windows: [
        { h: "Download Corvus-windows.zip", crow: t ? "zip with Corvus.bat + the kernel; QEMU via winget." : "big button. a zip appears in Downloads. we'll open it together.",
          body: "Save it anywhere.", shot: SHOTS.winExtract },
        { h: "Extract it", crow: "right-click, Extract All. Windows loves a ceremony.", body: "Right-click the zip → <b>Extract All…</b> → <b>Extract</b>.", shot: SHOTS.winExtract },
        { h: "Run Corvus.bat", crow: t ? "SmartScreen: More info → Run anyway." : "Windows will clutch its pearls. click 'More info', then 'Run anyway'. it's fine. it's a crow.",
          body: "Double-click <b>Corvus.bat</b>.", shot: SHOTS.smartscreen },
        { h: "Say yes to QEMU", crow: "it asks to install QEMU with winget. type y, press Enter, wait a minute.", body: "QEMU is the free virtual machine Corvus runs in.", shot: SHOTS.winget },
        { h: "Corvus boots", crow: "gold letters, purple button. you made it.", body: goalLine(p), shot: SHOTS.console },
      ],
      linux: [
        { h: "Copy the install command", crow: t ? "POSIX sh, reads fine. read it first, you're that kind of person." : "tap 'copy'. we're about to paste it somewhere scary. it's fine.",
          body: copyable("curl -fsSL " + INSTALL + " | sh"), shot: "" },
        { h: "Paste it in a terminal", crow: t ? "it sudo's your package manager for QEMU, nothing else." : "open Terminal, paste, Enter. it may ask your password: that's to install QEMU, the VM I live in.",
          body: "It installs QEMU, puts the kernel in <code>~/.local/share/corvus</code> and a <code>corvus</code> command in <code>~/.local/bin</code>.", shot: SHOTS.linuxTerm },
        { h: "Run corvus", crow: "type corvus. caw back at me.", body: copyable("corvus") + "<br>Update later: <code>corvus update</code>. Uninstall: <code>rm -rf ~/.local/share/corvus ~/.local/bin/corvus</code>", shot: SHOTS.console },
      ],
    };
    var primary = {
      ios: { label: "Open the web app", href: app, sub: "runs in Safari, works offline once loaded" },
      android: { label: "Open the web app", href: app, sub: "runs in Chrome, works offline once loaded" },
      mac: { label: "Download Corvus-mac.zip", href: REL + "Corvus-mac.zip?profile=" + encode(p), sub: "macOS 11+, Apple silicon or Intel" },
      windows: { label: "Download Corvus-windows.zip", href: REL + "Corvus-windows.zip?profile=" + encode(p), sub: "Windows 10/11, 64-bit" },
      linux: { label: "Copy the install command", href: "#", copy: "curl -fsSL " + INSTALL + " | sh", sub: "Linux and other Unix: QEMU from your package manager" },
    }[p.device];
    return { steps: G[p.device], primary: primary, desktop: ["mac", "windows", "linux"].indexOf(p.device) >= 0, app: app };
  }

  var current = null, stepIndex = 0;
  function finish() {
    var p = { device: answers.device, goal: answers.goal, tech: answers.tech };
    save(p);
    history.replaceState(null, "", "?profile=" + encode(p));
    show(p);
  }

  function show(p) {
    answers = { device: p.device, goal: p.goal, tech: p.tech };
    current = guide(p);
    $("ask").hidden = true;
    $("result").hidden = false;
    var a = $("primary");
    a.textContent = current.primary.label;
    a.href = current.primary.href;
    a.onclick = current.primary.copy ? function (e) { e.preventDefault(); copy(current.primary.copy, a, current.primary.label); } : null;
    $("primary-sub").textContent = current.primary.sub;
    $("webapp").href = current.app;
    $("vm").innerHTML = current.desktop
      ? '<div class="vm"><h4>Run in VM (recommended)</h4>Corvus boots inside QEMU, a free virtual machine, in its own window. Your disk and your OS are not touched. Delete it and it\'s gone.</div>' +
        '<div class="dual"><h4>Dual boot (experimental, coming)</h4>Not yet, honestly. Corvus only knows QEMU\'s virtual hardware today: no drivers for real disks, GPUs or keyboards. A dual-boot build would boot to a black screen. When it exists, it lands here.</div>'
      : "";
    step(0);
  }

  function step(i) {
    var steps = current.steps;
    stepIndex = Math.max(0, Math.min(i, steps.length - 1));
    var s = steps[stepIndex];
    $("guide-label").textContent = "step " + (stepIndex + 1) + " of " + steps.length;
    $("guide-bar").style.width = ((stepIndex + 1) / steps.length * 100) + "%";
    $("guide").innerHTML = '<div class="step"><h3>' + (stepIndex + 1) + ". " + s.h + "</h3><p>" + s.body + "</p>" + (s.shot || "") + "</div>";
    say(s.crow);
    $("prev").style.visibility = stepIndex === 0 ? "hidden" : "visible";
    $("next").textContent = stepIndex === steps.length - 1 ? "done ✓" : "next step →";
  }
  $("prev").addEventListener("click", function () { step(stepIndex - 1); });
  $("next").addEventListener("click", function () {
    if (stepIndex < current.steps.length - 1) step(stepIndex + 1);
    else say("that's it. go. the lantern's lit. (you can come back here any time.)");
  });

  function copy(text, el, label) {
    function ok() { el.textContent = "copied!"; setTimeout(function () { el.textContent = label; }, 1200); }
    if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(text).then(ok, function () { prompt("copy this:", text); });
    else prompt("copy this:", text);
  }
  document.addEventListener("click", function (e) {
    var b = e.target.closest && e.target.closest("button.copy");
    if (b) copy(b.getAttribute("data-copy"), b, "copy");
  });

  // ---- start -------------------------------------------------------------
  var saved = load();
  if (saved) {
    show(saved);
    say("caw, welcome back. same plan as last time? (or change your answers below.)");
  } else {
    ask(0);
  }

  // Everything live: the title cycles colours, the clock ticks.
  var PAL = ["#8c6fd1", "#c9a7ff", "#ffd166", "#8fe8f0", "#7fe0a8", "#ff7a70", "#f2eefa"];
  var title = $("title");
  title.innerHTML = "CORVUS".split("").map(function (c) { return "<span>" + c + "</span>"; }).join("");
  var spans = title.querySelectorAll("span");
  var t = 0;
  setInterval(function () {
    t++;
    for (var i = 0; i < spans.length; i++) spans[i].style.color = PAL[(i + t) % PAL.length];
    $("clock").textContent = new Date().toTimeString().slice(0, 8);
  }, 300);

  if ("serviceWorker" in navigator) navigator.serviceWorker.register("sw.js").catch(function () {});
})();
