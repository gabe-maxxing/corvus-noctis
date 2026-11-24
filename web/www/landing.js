const REL = "https://github.com/gabe-maxxing/corvus-noctis/releases/download/nightly/";
const INSTALL = new URL("install.sh", location.href).href;

const vm = (steps) => `<div class="box ok"><h2>Run in VM (recommended)</h2>
  <ol>${steps.map((s) => `<li>${s}</li>`).join("")}</ol>
  Corvus boots inside QEMU, a free virtual machine, in its own window. Your disk and your OS are not touched.</div>`;
const dual = `<div class="box soon"><h2>Dual boot (experimental, coming)</h2>
  Not yet, honestly. Corvus only knows QEMU's virtual hardware today: no drivers for real disks,
  GPUs or keyboards. A dual-boot build would boot to a black screen. When it exists, it lands here.</div>`;
const home = (how) => `<div class="box ok"><h2>Add to Home Screen</h2>
  <ol>${how.map((s) => `<li>${s}</li>`).join("")}</ol>
  Then Corvus opens fullscreen like an app, with its own icon.</div>`;

const OS = {
  ios: { label: "Open web app", href: "app/", sub: "runs in Safari, no install",
    html: home(["Open the web app.", "Tap the Share button (square with an arrow).", "Tap <b>Add to Home Screen</b>, then <b>Add</b>."]) },
  android: { label: "Open web app", href: "app/", sub: "runs in Chrome, no install",
    html: home(["Open the web app.", "Tap the <b>⋮</b> menu in Chrome.", "Tap <b>Add to Home screen</b> (or <b>Install app</b>)."]) },
  mac: { label: "Download Corvus-mac.zip", href: REL + "Corvus-mac.zip", sub: "macOS 11+, Apple silicon or Intel",
    html: vm(["Unzip and drag <b>Corvus.app</b> to Applications.", "First launch: right-click &gt; <b>Open</b> (not signed yet).",
      "No QEMU yet? Corvus offers <code>brew install qemu</code>."]) + dual },
  windows: { label: "Download Corvus-windows.zip", href: REL + "Corvus-windows.zip", sub: "Windows 10/11, 64-bit",
    html: vm(["Unzip anywhere.", "Double-click <b>Corvus.bat</b>.",
      "No QEMU yet? It offers <code>winget install SoftwareFreedomConservancy.QEMU</code>."]) + dual },
  linux: { label: "Copy the install command", href: "#", sub: "Linux and other Unix: QEMU from your package manager",
    html: vm([`Run: <code>curl -fsSL ${INSTALL} | sh</code><button class="copy" data-copy="curl -fsSL ${INSTALL} | sh">copy</button>`,
      "Then: <code>corvus</code> (update later with <code>corvus update</code>).",
      "Uninstall: <code>rm -rf ~/.local/share/corvus ~/.local/bin/corvus</code>"]) + dual },
};

function detect() {
  const ua = navigator.userAgent;
  if (/iPhone|iPad|iPod/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1)) return "ios";
  if (/Android/.test(ua)) return "android";
  if (/Macintosh|Mac OS X/.test(ua)) return "mac";
  if (/Windows/.test(ua)) return "windows";
  if (/Linux|X11|BSD|SunOS/.test(ua)) return "linux";
  return "ios"; // unknown: the web app works everywhere
}

function show(os) {
  const o = OS[os];
  const a = document.getElementById("primary");
  a.textContent = o.label;
  a.href = o.href;
  a.onclick = os === "linux" ? (e) => { e.preventDefault(); copy(`curl -fsSL ${INSTALL} | sh`, a, o.label); } : null;
  document.getElementById("primary-sub").textContent = o.sub;
  document.getElementById("details").innerHTML = o.html;
  for (const l of document.querySelectorAll("#others a[data-os]")) l.classList.toggle("on", l.dataset.os === os);
}

function copy(text, el, label) {
  if (!navigator.clipboard) { prompt("copy this:", text); return; }
  navigator.clipboard.writeText(text).then(() => { el.textContent = "copied!"; setTimeout(() => (el.textContent = label), 1200); },
    () => prompt("copy this:", text));
}

document.addEventListener("click", (e) => {
  const b = e.target.closest("button.copy");
  if (b) copy(b.dataset.copy, b, "copy");
  const n = e.target.closest("#others a[data-os]");
  if (n) { e.preventDefault(); show(n.dataset.os); history.replaceState(null, "", "#" + n.dataset.os); }
});

show(OS[location.hash.slice(1)] ? location.hash.slice(1) : detect());

// Everything live: the title cycles through the 16 colours, the clock ticks, the crow blinks.
const title = document.getElementById("title");
title.innerHTML = [..."CORVUS"].map((c) => `<span>${c}</span>`).join("");
const spans = title.querySelectorAll("span");
let t = 0;
setInterval(() => {
  t++;
  spans.forEach((s, i) => (s.style.color = `var(--c${9 + ((i + t) % 7)})`));
  document.getElementById("clock").textContent = new Date().toTimeString().slice(0, 8);
  document.getElementById("crow").textContent = t % 12 === 0
    ? "   .__\n   (- )>\n   / )\n  /_/" : "   .__\n   (o )>\n   / )\n  /_/";
}, 250);

if ("serviceWorker" in navigator) navigator.serviceWorker.register("sw.js").catch(function () {});
