; Ganchos do instalador NSIS gerado pelo Tauri.
; O próprio orbita.exe sabe se registrar e se remover (veja src/registration.rs).

!macro NSIS_HOOK_POSTINSTALL
  ; Item "Converter com Órbita" no botão direito + iniciar com o Windows.
  ExecWait '"$INSTDIR\orbita.exe" --register'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Numa atualização o item continua registrado; só remove na desinstalação.
  ${If} $UpdateMode <> 1
    ExecWait '"$INSTDIR\orbita.exe" --unregister'
  ${EndIf}
!macroend
