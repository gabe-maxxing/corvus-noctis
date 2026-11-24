// dio <question>: ask Diogenes of the lantern.
// Default: WebLLM in the browser (pinned version and model, loaded on first use).
// Fallback: a local Ollama at http://localhost:11434 when it answers.
// Persona: the "System prompt" section of diogenes/DIOGENES.md (copied next to
// this file by the Pages build as diogenes.md).

const WEBLLM_URL = "https://esm.run/@mlc-ai/web-llm@0.2.89";
export const MODEL = "Qwen2.5-0.5B-Instruct-q4f16_1-MLC";
const OLLAMA = "http://localhost:11434";

const FALLBACK_PROMPT =
  "You are Diogenes of Sinope, the Cynic, living inside Corvus OS with a lantern in daylight. " +
  "Answer in at most three short sentences: blunt, witty, never cruel. Never invent quotes.";

let persona = null;
let engine = null;
let loading = null;

async function systemPrompt() {
  if (persona) return persona;
  try {
    const md = await (await fetch("diogenes.md", { cache: "no-cache" })).text();
    const section = md.split(/^## System prompt\s*$/m)[1]?.split(/^## /m)[0]?.trim();
    persona = section || FALLBACK_PROMPT;
  } catch {
    persona = FALLBACK_PROMPT;
  }
  return persona;
}

async function hasWebGPU() {
  try {
    return !!(navigator.gpu && (await navigator.gpu.requestAdapter()));
  } catch {
    return false;
  }
}

async function ollamaModel() {
  try {
    const res = await fetch(`${OLLAMA}/api/tags`, { signal: AbortSignal.timeout(800) });
    const { models } = await res.json();
    return models?.[0]?.name || null;
  } catch {
    return null;
  }
}

async function loadWebLLM(progress) {
  if (engine) return engine;
  loading ??= (async () => {
    const webllm = await import(WEBLLM_URL);
    engine = await webllm.CreateMLCEngine(MODEL, {
      initProgressCallback: (report) => progress(report.progress ?? 0, report.text ?? ""),
    });
    return engine;
  })();
  try {
    return await loading;
  } catch (error) {
    loading = null;
    throw error;
  }
}

async function* streamWebLLM(messages) {
  const chunks = await engine.chat.completions.create({ messages, stream: true, max_tokens: 200, temperature: 0.8 });
  for await (const chunk of chunks) {
    const text = chunk.choices[0]?.delta?.content;
    if (text) yield text;
  }
}

async function* streamOllama(model, messages) {
  const res = await fetch(`${OLLAMA}/api/chat`, {
    method: "POST",
    body: JSON.stringify({ model, messages, stream: true, options: { num_predict: 200, temperature: 0.8 } }),
  });
  if (!res.ok) throw new Error(`ollama ${res.status}`);
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let nl;
    while ((nl = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, nl).trim();
      buffer = buffer.slice(nl + 1);
      if (!line) continue;
      const msg = JSON.parse(line);
      if (msg.message?.content) yield msg.message.content;
      if (msg.done) return;
    }
  }
}

/// Streams Diogenes' answer. `progress(fraction, text)` reports the first model load.
/// Returns the backend used: "webllm" or "ollama:<model>".
export async function ask(question, onText, progress) {
  const messages = [
    { role: "system", content: await systemPrompt() },
    { role: "user", content: question },
  ];
  let reason = "no WebGPU in this browser";
  if (engine || (await hasWebGPU())) {
    try {
      await loadWebLLM(progress);
      for await (const text of streamWebLLM(messages)) onText(text);
      return "webllm";
    } catch (error) {
      reason = `WebLLM failed (${error?.message || error})`;
    }
  }
  const model = await ollamaModel();
  if (model) {
    for await (const text of streamOllama(model, messages)) onText(text);
    return `ollama:${model}`;
  }
  throw new Error(`${reason}, and no Ollama at localhost:11434`);
}
