import { describe, expect, it } from "bun:test";
import { unwrapInvokeEnvelope } from "../client";
import { SdkError } from "../errors";

describe("unwrapInvokeEnvelope", () => {
  it("returns data for successful envelope", () => {
    const output = unwrapInvokeEnvelope("notes", "list-notes", {
      ok: true,
      data: { count: 1 },
    });

    expect(output).toEqual({ count: 1 });
  });

  it("throws tool_error for ok:false envelope", () => {
    expect(() =>
      unwrapInvokeEnvelope("notes", "list-notes", {
        ok: false,
        error: { code: "bad_request", message: "invalid query" },
      }),
    ).toThrow(SdkError);

    try {
      unwrapInvokeEnvelope("notes", "list-notes", {
        ok: false,
        error: { code: "bad_request", message: "invalid query" },
      });
    } catch (error) {
      expect((error as SdkError).code).toBe("tool_error");
      expect((error as SdkError).message).toContain("invalid query");
    }
  });
});
