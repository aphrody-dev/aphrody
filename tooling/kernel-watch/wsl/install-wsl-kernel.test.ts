import { describe, expect, test } from "bun:test";
import { mergeWslConfig, sha256Hex, wslPath } from "./install-wsl-kernel.ts";

describe("install-wsl-kernel", () => {
  test("doubles backslashes for .wslconfig", () => {
    expect(wslPath("C:/Users/me/wsl-kernel/bzImage")).toBe("C:\\\\Users\\\\me\\\\wsl-kernel\\\\bzImage");
    expect(wslPath("C:\\Users\\me\\bzImage")).toBe("C:\\\\Users\\\\me\\\\bzImage");
  });

  test("creates the [wsl2] section in an empty file", () => {
    expect(mergeWslConfig("", { kernel: "C:\\\\k\\\\bzImage" })).toBe("[wsl2]\nkernel=C:\\\\k\\\\bzImage\n");
  });

  test("replaces existing keys in place and keeps every other line", () => {
    const before = "[experimental]\nautoMemoryReclaim=gradual\n\n[wsl2]\nmemory=8GB\nKernel=C:\\\\old\\\\bzImage\nswap=0\n\n[interop]\nenabled=true\n";
    const after = mergeWslConfig(before, { kernel: "C:\\\\new\\\\bzImage", kernelModules: "C:\\\\new\\\\modules.vhdx", kernelCommandLine: "quiet" });
    expect(after).toBe("[experimental]\nautoMemoryReclaim=gradual\n\n[wsl2]\nmemory=8GB\nkernel=C:\\\\new\\\\bzImage\nswap=0\nkernelModules=C:\\\\new\\\\modules.vhdx\nkernelCommandLine=quiet\n\n[interop]\nenabled=true\n");
  });

  test("appends [wsl2] after other sections and keeps CRLF files CRLF", () => {
    const after = mergeWslConfig("[interop]\r\nenabled=true\r\n", { kernel: "K" });
    expect(after).toBe("[interop]\r\nenabled=true\r\n\r\n[wsl2]\r\nkernel=K\r\n");
  });

  test("hashes", () => {
    expect(sha256Hex(new TextEncoder().encode("abc"))).toBe("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  });
});
