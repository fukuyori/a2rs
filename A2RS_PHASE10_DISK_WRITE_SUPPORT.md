# A2RS Phase10 Disk II Write Support

## Scope
- nibble write gate driven by strict sequencer timing
- write protect sense and write blocking
- dirty disk tracking
- eject-time flush back to the original image file
- original format aware write-back (`.dsk`, `.po`, `.nib`)

## Current behavior
- write path continues to use the existing nibble/tick pipeline
- modified disks are marked dirty
- eject now performs a best-effort save back to the source filename
- write-protected disks refuse writeback

## Notes
- `.dsk` and `.po` are rebuilt from the internal NIB image by sector decode
- `.nib` is written back verbatim
- no NTSC/video changes are included in this phase
