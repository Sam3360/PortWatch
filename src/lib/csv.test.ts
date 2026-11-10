import { describe, expect, it } from "vitest";
import { escapeField, toCsv } from "./csv";

describe("csv", () => {
  it("leaves plain values alone", () => {
    expect(escapeField("chrome.exe")).toBe("chrome.exe");
    expect(escapeField(null)).toBe("");
    expect(escapeField(443)).toBe("443");
  });
  it("quotes commas, quotes and newlines", () => {
    expect(escapeField("a,b")).toBe('"a,b"');
    expect(escapeField('say "hi"')).toBe('"say ""hi"""');
    expect(escapeField("a\nb")).toBe('"a\nb"');
  });
  it("neutralises formulas but keeps numbers", () => {
    expect(escapeField("=1+1")).toBe("'=1+1");
    expect(escapeField("@SUM")).toBe("'@SUM");
    expect(escapeField("-12")).toBe("-12");
  });
  it("writes CRLF rows with a header", () => {
    expect(toCsv(["A", "B"], [[1, "x,y"]])).toBe('A,B\r\n1,"x,y"\r\n');
  });
});
