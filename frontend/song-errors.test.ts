import { expect, it } from "vitest";
import { songErrorKey } from "./song-errors";

it("recognizes new and stored German and English timeout messages", () => {
  for (const message of [
    "Synthese: Zeitüberschreitung nach 2 Min. In den Einstellungen die Timeout-Zeit erhöhen.",
    "LM: Zeitüberschreitung nach 1800s",
    "lm: time limit exceeded",
    "Timeout after 30 minutes. Raise the timeout in the settings.",
    "operation timed out",
    "request timeout while generating codes",
  ]) expect(songErrorKey(message)).toBe("err_timeout");
});

it("retains the existing summaries for other failures", () => {
  expect(songErrorKey("Server nicht erreichbar")).toBe("err_offline");
  expect(songErrorKey("Language model returned no audio codes")).toBe("err_codes");
  expect(songErrorKey("Operation cancelled")).toBe("err_details");
  expect(songErrorKey("failed to allocate")).toBe("err_details");
  expect(songErrorKey()).toBe("err_details");
});
