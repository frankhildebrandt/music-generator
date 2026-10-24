// Stored messages stay intact; this selects the localized song-list summary.
export function songErrorKey(message = ""): "err_offline" | "err_codes" | "err_timeout" | "err_details" {
  if (/Zeitüberschreitung|time limit exceeded|timed?\s*out|timeout/i.test(message)) return "err_timeout";
  if (/nicht erreichbar/i.test(message)) return "err_offline";
  if (/Codes/i.test(message)) return "err_codes";
  return "err_details";
}
