import { SdkError } from "./errors";

export interface ParsedSseEvent {
  event: string;
  data: unknown;
  raw: string;
}

function parseSseBlock(raw: string): ParsedSseEvent | null {
  if (!raw.trim()) return null;

  let event = "message";
  const dataLines: string[] = [];

  for (const line of raw.split("\n")) {
    if (line.startsWith(":")) {
      continue;
    }
    if (line.startsWith("event:")) {
      event = line.slice("event:".length).trim();
      continue;
    }
    if (line.startsWith("data:")) {
      dataLines.push(line.slice("data:".length).trimStart());
    }
  }

  const rawData = dataLines.join("\n");
  if (!rawData) {
    return null;
  }

  let data: unknown = rawData;
  try {
    data = JSON.parse(rawData);
  } catch {
    data = rawData;
  }

  return { event, data, raw: rawData };
}

export async function* readSse(response: Response): AsyncGenerator<ParsedSseEvent> {
  if (!response.body) {
    throw new SdkError({
      code: "sse_error",
      message: "Missing response body for SSE stream",
    });
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) {
      break;
    }

    buffer += decoder.decode(value, { stream: true });
    buffer = buffer.replace(/\r\n/g, "\n");

    let splitIndex = buffer.indexOf("\n\n");
    while (splitIndex !== -1) {
      const block = buffer.slice(0, splitIndex);
      buffer = buffer.slice(splitIndex + 2);

      const parsed = parseSseBlock(block);
      if (parsed) {
        yield parsed;
      }

      splitIndex = buffer.indexOf("\n\n");
    }
  }

  if (buffer.trim()) {
    const parsed = parseSseBlock(buffer);
    if (parsed) {
      yield parsed;
    }
  }
}
