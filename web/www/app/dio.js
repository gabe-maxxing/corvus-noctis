// dio <question>: ask Diogenes of the lantern.
// Default: WebLLM in the browser (pinned version and model, loaded on first use).
// Fallback: a local Ollama at http://localhost:11434 when it answers.
// Persona: the "System prompt" section of diogenes/DIOGENES.md (copied next to
// this file by the Pages build as diogenes.md).
// Plain ES2017 so the console still loads on iOS 12 (where dio explains what's missing).

var WEBLLM_URL = "https://esm.run/@mlc-ai/web-llm@0.2.89";
export var MODEL = "Qwen2.5-0.5B-Instruct-q4f16_1-MLC";
var OLLAMA = "http://localhost:11434";

var FALLBACK_PROMPT =
  "You are Diogenes of Sinope, the Cynic, living inside Corvus OS with a lantern in daylight. " +
  "Answer in at most three short sentences: blunt, witty, never cruel. Never invent quotes.";

// A runtime import() hidden from the ES2017 parse, only reached on browsers with WebGPU.
var dynamicImport = new Function("url", "return import(url)");

var persona = null;
var engine = null;
var loading = null;

function withTimeout(promise, ms) {
  return new Promise(function (resolve, reject) {
    var timer = setTimeout(function () { reject(new Error("timeout")); }, ms);
    promise.then(function (v) { clearTimeout(timer); resolve(v); }, function (e) { clearTimeout(timer); reject(e); });
  });
}

function systemPrompt() {
  if (persona) return Promise.resolve(persona);
  return fetch("diogenes.md", { cache: "no-cache" })
    .then(function (res) { return res.text(); })
    .then(function (md) {
      var parts = md.split(/^## System prompt\s*$/m);
      var section = parts.length > 1 ? parts[1].split(/^## /m)[0].trim() : "";
      persona = section || FALLBACK_PROMPT;
      return persona;
    }, function () {
      persona = FALLBACK_PROMPT;
      return persona;
    });
}

function hasWebGPU() {
  if (!navigator.gpu) return Promise.resolve(false);
  return navigator.gpu.requestAdapter().then(function (a) { return !!a; }, function () { return false; });
}

function ollamaModel() {
  return withTimeout(fetch(OLLAMA + "/api/tags"), 800)
    .then(function (res) { return res.json(); })
    .then(function (body) { return body.models && body.models[0] ? body.models[0].name : null; })
    .catch(function () { return null; });
}

function loadWebLLM(progress) {
  if (engine) return Promise.resolve(engine);
  if (!loading) {
    loading = dynamicImport(WEBLLM_URL).then(function (webllm) {
      return webllm.CreateMLCEngine(MODEL, {
        initProgressCallback: function (report) { progress(report.progress || 0, report.text || ""); },
      });
    }).then(function (e) {
      engine = e;
      return e;
    }, function (error) {
      loading = null;
      throw error;
    });
  }
  return loading;
}

function streamWebLLM(messages, onText) {
  return engine.chat.completions.create({ messages: messages, stream: true, max_tokens: 200, temperature: 0.8 })
    .then(function (chunks) {
      var it = chunks[Symbol.asyncIterator]();
      function next() {
        return it.next().then(function (step) {
          if (step.done) return;
          var choice = step.value.choices[0];
          if (choice && choice.delta && choice.delta.content) onText(choice.delta.content);
          return next();
        });
      }
      return next();
    });
}

function streamOllama(model, messages, onText) {
  return fetch(OLLAMA + "/api/chat", {
    method: "POST",
    body: JSON.stringify({ model: model, messages: messages, stream: true, options: { num_predict: 200, temperature: 0.8 } }),
  }).then(function (res) {
    if (!res.ok) throw new Error("ollama " + res.status);
    var reader = res.body.getReader();
    var decoder = new TextDecoder();
    var buffer = "";
    function pump() {
      return reader.read().then(function (step) {
        if (step.done) return;
        buffer += decoder.decode(step.value, { stream: true });
        var nl;
        while ((nl = buffer.indexOf("\n")) >= 0) {
          var line = buffer.slice(0, nl).trim();
          buffer = buffer.slice(nl + 1);
          if (!line) continue;
          var msg = JSON.parse(line);
          if (msg.message && msg.message.content) onText(msg.message.content);
          if (msg.done) return;
        }
        return pump();
      });
    }
    return pump();
  });
}

/// Streams Diogenes' answer. `progress(fraction, text)` reports the first model load.
/// Resolves to the backend used: "webllm" or "ollama:<model>".
export function ask(question, onText, progress) {
  var messages;
  var reason = "no WebGPU in this browser";
  return systemPrompt().then(function (prompt) {
    messages = [{ role: "system", content: prompt }, { role: "user", content: question }];
    return engine ? true : hasWebGPU();
  }).then(function (gpu) {
    if (!gpu) return null;
    return loadWebLLM(progress)
      .then(function () { return streamWebLLM(messages, onText); })
      .then(function () { return "webllm"; }, function (error) {
        reason = "WebLLM failed (" + (error && error.message ? error.message : error) + ")";
        return null;
      });
  }).then(function (backend) {
    if (backend) return backend;
    return ollamaModel().then(function (model) {
      if (!model) throw new Error(reason + ", and no Ollama at localhost:11434");
      return streamOllama(model, messages, onText).then(function () { return "ollama:" + model; });
    });
  });
}
