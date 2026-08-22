import { invoke } from "@tauri-apps/api/core";
import type { OverlaySettings } from "@/shared/lib/types/pipeline";

export const overlayHide = () => invoke("overlay_hide");
export const overlayPreview = () => invoke("overlay_preview", { mock: true });
export const overlayIsPreviewMode = () => invoke<boolean>("overlay_is_preview_mode");
export const overlaySetPointerInteractive = (interactive: boolean) =>
  invoke("overlay_set_pointer_interactive", { interactive });
export const overlaySetOffset = (offsetX: number, offsetY: number) =>
  invoke("overlay_set_offset", { offsetX, offsetY });
export const overlaySetSize = (width: number, height: number) =>
  invoke("overlay_set_size", { width, height });

export type OverlaySettingsPatch = {
  opacity?: number;
  clickThrough?: boolean;
};

export const overlayPatchSettings = (patch: OverlaySettingsPatch) =>
  invoke<OverlaySettings>("overlay_patch_settings", { patch });
