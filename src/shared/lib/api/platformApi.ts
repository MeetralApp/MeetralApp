import { invoke } from "@tauri-apps/api/core";

/** OS platform id from the Tauri backend (`windows` | `macos` | …). */
export async function getPlatform(): Promise<string> {
  return invoke<string>("get_platform");
}
