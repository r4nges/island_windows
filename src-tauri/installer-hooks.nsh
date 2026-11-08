; Registra o pacote esparso (identidade para ler notificações).
; Precisa do Modo de Desenvolvedor; se falhar, o Island funciona normalmente
; e oferece "Ativar" na aba Notificações.
!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -Command "Add-AppxPackage -Register \"$INSTDIR\AppxManifest.xml\" -ExternalLocation \"$INSTDIR\" -ForceUpdateFromAnyVersion"'
!macroend

; Numa atualização o instalador novo roda este desinstalador com /UPDATE
; ($UpdateMode = 1): aí "Iniciar com o Windows" e o marcador ficam, senão a
; escolha do usuário se perderia a cada versão. Só a desinstalação de verdade
; limpa a entrada do Run (que apontaria para um exe apagado) e o marcador
; (reinstalar volta ao padrão: ligado).
!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-AppxPackage r4nges.Island | Remove-AppxPackage"'
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Island"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Island"
    Delete "$APPDATA\dev.r4nges.island\autostart-default"
  ${EndIf}
!macroend
