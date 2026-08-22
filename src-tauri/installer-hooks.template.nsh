; Template for installer-hooks.nsh — rendered by Build-SparsePackage.ps1 with
; @SPARSE_PAYLOAD_DIR@ replaced by the absolute staged payload path.
; (Absolute paths are required: NSIS resolves ${__FILEDIR__} to the main script's
; workdir under target/, not to this file's directory.)
;
; Ships the signed sparse MSIX + logos + runtime scripts next to meetral.exe and
; registers package identity per-user. Registration failure is non-fatal: the app
; runs fine without identity, only Task Manager grouping / notifications differ.
; Logos must sit next to the exe because sparse packages resolve ms-appx:///
; asset URIs at the ExternalLocation folder, not inside the .msix.

!macro NSIS_HOOK_POSTINSTALL
  SetOutPath "$INSTDIR"
  File "@SPARSE_PAYLOAD_DIR@\Meetral-sparse.msix"
  ; MeetralAppList / MeetralTile + targetsize altform-unplated/lightunplated
  ; variants + resources.pri (required at ExternalLocation for unplated taskbar).
  File "@SPARSE_PAYLOAD_DIR@\Meetral*.png"
  File "@SPARSE_PAYLOAD_DIR@\resources.pri"
  File "@SPARSE_PAYLOAD_DIR@\Register-MeetralIdentity.ps1"
  File "@SPARSE_PAYLOAD_DIR@\Unregister-MeetralIdentity.ps1"
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\Register-MeetralIdentity.ps1" -InstallDir "$INSTDIR"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\Unregister-MeetralIdentity.ps1"'
!macroend
