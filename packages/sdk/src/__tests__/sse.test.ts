import { describe, expect, it } from "bun:test";
import { readSse } from "../sse";

describe("readSse", () => {
  it("parses chunked SSE frames", async () => {
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("event: part.updated\ndata: {\"message_id\":\"m1\",\"part_id\":\"p1\""));
        controller.enqueue(new TextEncoder().encode(",\"part\":{\"type\":\"text\"}}\n\n"));
        controller.enqueue(new TextEncoder().encode("event: done\ndata: {\"context\":{\"session_id\":\"s1\"},\"messages\":[]}\n\n"));
        controller.close();
      },
    });

    const response = new Response(stream);
    const events: Array<{ event: string; data: unknown }> = [];

    for await (const event of readSse(response)) {
      events.push({ event: event.event, data: event.data });
    }

    expect(events.length).toBe(2);
    expect(events[0]?.event).toBe("part.updated");
    expect((events[0]?.data as { part_id: string }).part_id).toBe("p1");
    expect(events[1]?.event).toBe("done");
  });
});
