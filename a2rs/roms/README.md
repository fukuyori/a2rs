# ROM Files

A2RS requires the following ROM files to run:

## Required ROMs

### Apple II System ROM
- `apple2.rom` (12KB or 16KB) - Apple II/II+ ROM
- `apple2e.rom` (32KB) - Apple IIe ROM

These files are copyrighted by Apple and must be obtained legally.

## Optional ROMs

### Disk II Boot ROM
- `disk2.rom` (256 bytes) - Disk II controller boot ROM

**Note:** If disk2.rom is not provided, A2RS will use VBR (Virtual Boot ROM) 
mode, which can boot DSK format disk images without the actual ROM.
VBR mode does NOT include any copyrighted Apple code - it implements the
boot functionality algorithmically.

NIB and WOZ format disk images require the actual disk2.rom file.

## Legal Notice

ROM files are copyrighted by Apple Inc. and are not included with A2RS.
You must obtain these files legally, such as by:
- Dumping from your own Apple II hardware
- Using legally obtained ROM images

A2RS source code is MIT licensed and contains no Apple copyrighted code.
