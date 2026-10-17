; Current-user autostart ownership. Test harnesses override only these registry roots.
!ifndef SSR2CLASH_RUN_KEY
  !define SSR2CLASH_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!endif
!ifndef SSR2CLASH_APPROVED_KEY
  !define SSR2CLASH_APPROVED_KEY "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
!endif
Var SSR2ClashPreviousAutostart
Var SSR2ClashMigrateAutostart

!macro NSIS_HOOK_PREINSTALL
  StrCpy $SSR2ClashPreviousAutostart ""
  StrCpy $SSR2ClashMigrateAutostart 0
  ReadRegStr $R0 HKCU "${SSR2CLASH_RUN_KEY}" "${PRODUCTNAME}"
  ReadRegStr $R1 SHCTX "${MANUPRODUCTKEY}" ""
  ${If} $R1 != ""
  ${AndIf} $R0 == '$\"$R1\${MAINBINARYNAME}.exe$\" --autostart'
    StrCpy $SSR2ClashPreviousAutostart $R0
    StrCpy $SSR2ClashMigrateAutostart 1
  ${EndIf}

  IfFileExists "$INSTDIR\portable.flag" 0 ssr2clash_check_writable
    MessageBox MB_ICONSTOP|MB_OK "This folder contains a portable copy. Choose a separate installation folder." /SD IDOK
    Abort
  ssr2clash_check_writable:
  ClearErrors
  CreateDirectory "$INSTDIR"
  IfErrors ssr2clash_not_writable
  GetTempFileName $R0 "$INSTDIR"
  IfErrors ssr2clash_not_writable
  Delete "$R0"
  Goto ssr2clash_folder_ready
  ssr2clash_not_writable:
    MessageBox MB_ICONSTOP|MB_OK "Cannot write to the selected installation folder. Choose a writable folder." /SD IDOK
    Abort
  ssr2clash_folder_ready:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $SSR2ClashMigrateAutostart == 1
    ReadRegStr $R0 HKCU "${SSR2CLASH_RUN_KEY}" "${PRODUCTNAME}"
    ${If} $R0 == $SSR2ClashPreviousAutostart
      WriteRegStr HKCU "${SSR2CLASH_RUN_KEY}" "${PRODUCTNAME}" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --autostart'
    ${EndIf}
  ${EndIf}
!macroend

!macro SSR2CLASH_REMOVE_OWN_AUTOSTART
  ${If} $UpdateMode <> 1
    ReadRegStr $R0 HKCU "${SSR2CLASH_RUN_KEY}" "${PRODUCTNAME}"
    ${If} $R0 == '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --autostart'
      DeleteRegValue HKCU "${SSR2CLASH_RUN_KEY}" "${PRODUCTNAME}"
      DeleteRegValue HKCU "${SSR2CLASH_APPROVED_KEY}" "${PRODUCTNAME}"
    ${EndIf}
  ${EndIf}
!macroend
