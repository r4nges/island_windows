; Registra o pacote esparso (identidade para ler notificações).
; Precisa do Modo de Desenvolvedor; se falhar, o Island funciona normalmente
; e oferece "Ativar" na aba Notificações.
!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -Command "Add-AppxPackage -Register \"$INSTDIR\AppxManifest.xml\" -ExternalLocation \"$INSTDIR\" -ForceUpdateFromAnyVersion"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-AppxPackage r4nges.Island | Remove-AppxPackage"'
!macroend
