import * as voiceApi from "@/shared/lib/api/voiceApi";

/**
* Voice list / preview / validate IPC for Settings.
* Owns the ElevenLabs + Soniox catalog surface formerly drilled from App → AppShell.
*/
export function useVoiceCatalog() {
  return {
    listElevenLabsVoices: voiceApi.listElevenLabsVoices,
    listElevenLabsModels: voiceApi.listElevenLabsModels,
    listSonioxVoices: voiceApi.listSonioxVoices,
    validateElevenLabsVoice: voiceApi.validateElevenLabsVoice,
    previewElevenLabsVoice: voiceApi.previewElevenLabsVoice,
    previewSonioxVoice: voiceApi.previewSonioxVoice,
    testElevenLabsApiKey: voiceApi.testElevenLabsApiKey,
    testFishAudioApiKey: voiceApi.testFishAudioApiKey,
    listFishAudioVoices: voiceApi.listFishAudioVoices,
    listFishAudioModels: voiceApi.listFishAudioModels,
    validateFishAudioVoice: voiceApi.validateFishAudioVoice,
    previewFishAudioVoice: voiceApi.previewFishAudioVoice,
  };
}
